mod model;

use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex, RwLock},
};

use anyhow::anyhow;
use cli::{chapter_downloader::download_chapter_pages_as_cbz, source::Source};
use mlua::prelude::*;
use model::Chapter;

use crate::model::Manga;

static SOURCE: Mutex<Option<Source>> = Mutex::new(None);
static SERIALIZATION_OPTIONS: LuaSerializeOptions =
    LuaSerializeOptions::new().serialize_none_to_null(false);

fn initialize(_: &Lua, sources_path: String) -> LuaResult<()> {
    let source = Source::from_aix_file(Path::new(format!("{}/source.aix", &sources_path).as_str()))
        .map_err(LuaError::external)?;

    *SOURCE.lock().unwrap() = Some(source);

    Ok(())
}

fn search_mangas(lua: &Lua, query: String) -> LuaResult<Vec<LuaValue>> {
    let mut binding = SOURCE.lock().unwrap();
    let source = binding
        .as_mut()
        .ok_or(anyhow!("could not read source (was initialize called?)"))
        .map_err(LuaError::external)?;

    let mangas: Vec<LuaValue> = source
        .search_mangas(query)
        .map_err(LuaError::external)?
        .iter()
        .map(|source_manga| Manga::from(source_manga.clone()))
        .map(|manga| lua.to_value_with(&manga, SERIALIZATION_OPTIONS).unwrap())
        .collect();

    Ok(mangas)
}

fn list_chapters(lua: &Lua, (source_id, manga_id): (String, String)) -> LuaResult<Vec<LuaValue>> {
    let mut binding = SOURCE.lock().unwrap();
    let source = binding
        .as_mut()
        .ok_or(anyhow!("could not read source (was initialize called?)"))
        .map_err(LuaError::external)?;

    let chapters: Vec<LuaValue> = source
        .get_chapter_list(manga_id)
        .map_err(LuaError::external)?
        .iter()
        .map(|source_chapter| Chapter::from(source_chapter.clone()))
        .map(|chapter| lua.to_value_with(&chapter, SERIALIZATION_OPTIONS).unwrap())
        .collect();

    Ok(chapters)
}

fn download_chapter(
    lua: &Lua,
    // FIXME we should manage the output path somehow
    (source_id, manga_id, chapter_id, output_path): (String, String, String, String),
) -> LuaResult<()> {
    let mut binding = SOURCE.lock().unwrap();
    let source = binding
        .as_mut()
        .ok_or(anyhow!("could not read source (was initialize called?)"))
        .map_err(LuaError::external)?;

    let pages = source
        .get_page_list(manga_id, chapter_id)
        .map_err(LuaError::external)?;
    let output_file = fs::File::create(output_path)?;
    download_chapter_pages_as_cbz(output_file, pages).map_err(LuaError::external)?;

    Ok(())
}

#[mlua::lua_module]
fn backend(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;
    exports.set("initialize", lua.create_function(initialize)?)?;
    exports.set("search_mangas", lua.create_function(search_mangas)?)?;
    exports.set("list_chapters", lua.create_function(list_chapters)?)?;
    exports.set("download_chapter", lua.create_function(download_chapter)?)?;

    Ok(exports)
}
