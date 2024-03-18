use anyhow::{anyhow, bail, Context, Result};
use scopeguard::defer;
use serde::Deserialize;
use std::{collections::HashMap, fs, io::Read, path::Path};
use wasmtime::*;
use zip::ZipArchive;

use self::{
    model::{Chapter, Filter, Manga, MangaPageResult, Page},
    wasm_imports::{
        aidoku::register_aidoku_imports, defaults::register_defaults_imports,
        env::register_env_imports, html::register_html_imports, json::register_json_imports,
        net::register_net_imports, std::register_std_imports,
    },
    wasm_store::{Context as StoreContext, ObjectValue, Value, WasmStore},
};

pub mod model;
mod wasm_imports;
mod wasm_store;

#[derive(Debug, Clone, Deserialize)]
pub struct SourceInfo {
    pub id: String,
    pub lang: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourceManifest {
    pub info: SourceInfo,
}

pub struct Source {
    store: Store<WasmStore>,
    instance: Instance,
}

impl Source {
    pub fn from_aix_file(path: &Path) -> Result<Self> {
        let file = fs::File::open(path)?;
        let mut archive = ZipArchive::new(file)?;

        let manifest_file = archive
            .by_name("Payload/source.json")
            .with_context(|| "while loading source.json")?;
        let manifest: SourceManifest = serde_json::from_reader(manifest_file)?;

        let mut wasm_file_contents: Vec<u8> = Vec::new();
        archive
            .by_name("Payload/main.wasm")
            .with_context(|| "while loading main.wasm")?
            .read_to_end(&mut wasm_file_contents)?;

        let engine = Engine::default();
        let wasm_store = WasmStore::new(manifest.info.id);
        let mut store = Store::new(&engine, wasm_store);
        let module = Module::from_binary(&engine, wasm_file_contents.as_slice())
            .with_context(|| format!("failed loading module from {}", path.display()))?;

        let mut linker = Linker::new(&engine);
        register_aidoku_imports(&mut linker)?;
        register_defaults_imports(&mut linker)?;
        register_env_imports(&mut linker)?;
        register_html_imports(&mut linker)?;
        register_json_imports(&mut linker)?;
        register_net_imports(&mut linker)?;
        register_std_imports(&mut linker)?;

        let instance = linker.instantiate(&mut store, &module).with_context(|| {
            format!(
                "failed creating instance when loading from {}",
                path.display()
            )
        })?;

        Ok(Self { store, instance })
    }

    pub fn get_manga_list(&mut self) -> Result<Vec<Manga>> {
        self.search_mangas_by_filters(vec![])
    }

    pub fn search_mangas(&mut self, query: String) -> Result<Vec<Manga>> {
        self.search_mangas_by_filters(vec![Filter::Title(query)])
    }

    fn search_mangas_by_filters(&mut self, filters: Vec<Filter>) -> Result<Vec<Manga>> {
        let wasm_function = self
            .instance
            .get_typed_func::<(i32, i32), (i32)>(&mut self.store, "get_manga_list")?;
        let filters_descriptor = self.store.data_mut().store_std_value(
            Value::Array(
                filters
                    .iter()
                    .map(|filter| Value::Object(ObjectValue::Filter(filter.clone())))
                    .collect::<Vec<_>>(),
            ),
            None,
        );

        // FIXME what if i actually want more pages tho
        let page = 1i32;
        let page_descriptor =
            wasm_function.call(&mut self.store, (filters_descriptor as i32, page))?;
        // TODO maybe use some `TryInto` implementation here to make things easier to read
        let mangas: Vec<Manga> = match self
            .store
            .data_mut()
            .read_std_value(page_descriptor as usize)
            .ok_or(anyhow!("could not read data from page descriptor"))?
        {
            Value::Object(ObjectValue::MangaPageResult(MangaPageResult { manga, .. })) => manga,
            other => bail!(
                "expected page descriptor to be an array, found {:?} instead",
                other
            ),
        };

        // TODO remove page_descriptor and filters_descriptor from the source's storage

        Ok(mangas)
    }

    pub fn get_chapter_list(&mut self, manga_id: String) -> Result<Vec<Chapter>> {
        // FIXME setting the context modifies some of the operations done inside the
        // Aidoku functions (mainly `create_chapter`/`create_page`)
        // not sure if i like this but i think its decent for now
        self.store.data_mut().context = StoreContext::Manga {
            id: manga_id.clone(),
        };
        let result = self.get_chapter_list_inner(manga_id);
        self.store.data_mut().context = StoreContext::None;

        result
    }

    fn get_chapter_list_inner(&mut self, manga_id: String) -> Result<Vec<Chapter>> {
        // HACK aidoku actually places the entire `Manga` object into the store, but it seems only
        // the `id` field is needed, so we just store a `HashMap` with the `id` set.
        // surely this wont break in the future!
        let mut manga_hashmap = HashMap::new();
        manga_hashmap.insert("id".to_string(), Value::String(manga_id));

        let manga_descriptor = self
            .store
            .data_mut()
            .store_std_value(Value::Object(ObjectValue::HashMap(manga_hashmap)), None);

        // FIXME what the fuck is chapter counter, aidoku sets it here
        let wasm_function = self
            .instance
            .get_typed_func::<(i32), (i32)>(&mut self.store, "get_chapter_list")?;
        let chapter_list_descriptor =
            wasm_function.call(&mut self.store, manga_descriptor as i32)?;

        let chapters: Vec<Chapter> = match self
            .store
            .data_mut()
            .read_std_value(chapter_list_descriptor as usize)
            .ok_or(anyhow!("could not read data from chapter list descriptor"))?
        {
            Value::Array(array) => array
                .into_iter()
                .map(|v| match v {
                    Value::Object(ObjectValue::Chapter(chapter)) => Some(chapter),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
                .ok_or(anyhow!("unexpected element in chapter array"))?,
            other => bail!(
                "expected page descriptor to be an array, found {:?} instead",
                other
            ),
        };

        Ok(chapters)
    }

    pub fn get_page_list(&mut self, manga_id: String, chapter_id: String) -> Result<Vec<Page>> {
        // FIXME setting the context modifies some of the operations done inside the
        // Aidoku functions (mainly `create_chapter`/`create_page`)
        // not sure if i like this but i think its decent for now
        self.store.data_mut().context = StoreContext::Chapter {
            manga_id: manga_id.clone(),
            id: chapter_id.clone(),
        };
        let result = self.get_page_list_inner(manga_id, chapter_id);
        self.store.data_mut().context = StoreContext::None;

        result
    }

    fn get_page_list_inner(&mut self, manga_id: String, chapter_id: String) -> Result<Vec<Page>> {
        // HACK the same thing with the `Manga` said above, we also only need the `id`
        // from the `Chapter` object
        // FIXME we could create an `ObjectValue` for this concept? something like
        // `MangaId` or `ChapterId` would be a cleaner implementation
        let mut chapter_hashmap = HashMap::new();
        chapter_hashmap.insert("id".to_string(), Value::String(chapter_id));
        chapter_hashmap.insert("mangaId".to_string(), Value::String(manga_id));

        let chapter_descriptor = self
            .store
            .data_mut()
            .store_std_value(Value::Object(ObjectValue::HashMap(chapter_hashmap)), None);

        // FIXME what the fuck is chapter counter, aidoku sets it here
        let wasm_function = self
            .instance
            .get_typed_func::<(i32), (i32)>(&mut self.store, "get_page_list")?;
        let page_list_descriptor =
            wasm_function.call(&mut self.store, chapter_descriptor as i32)?;

        dbg!(page_list_descriptor);

        let pages: Vec<Page> = match self
            .store
            .data_mut()
            .read_std_value(page_list_descriptor as usize)
            .ok_or(anyhow!("could not read data from page list descriptor"))?
        {
            Value::Array(array) => array
                .into_iter()
                .map(|v| match v {
                    Value::Object(ObjectValue::Page(page)) => Some(page),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
                .ok_or(anyhow!("unexpected element in page array"))?,
            other => bail!(
                "expected page descriptor to be an array, found {:?} instead",
                other
            ),
        };

        Ok(pages)
    }
}
