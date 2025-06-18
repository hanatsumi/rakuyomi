use anyhow::Result;

use scraper::{Element, Html as ScraperHtml, Node, Selector};
use url::Url;
use wasm_macros::{aidoku_wasm_function, register_wasm_function};
use wasmi::{Caller, Linker};

use crate::source::wasm_store::{HTMLElement, Html, Value, WasmStore};

pub fn register_html_imports(linker: &mut Linker<WasmStore>) -> Result<()> {
    register_wasm_function!(linker, "html", "parse", parse)?;
    register_wasm_function!(linker, "html", "parse_fragment", parse_fragment)?;
    register_wasm_function!(linker, "html", "parse_with_uri", parse_with_uri)?;
    register_wasm_function!(
        linker,
        "html",
        "parse_fragment_with_uri",
        parse_fragment_with_uri
    )?;

    register_wasm_function!(linker, "html", "select", select)?;
    register_wasm_function!(linker, "html", "attr", attr)?;

    register_wasm_function!(linker, "html", "set_text", set_text)?;
    register_wasm_function!(linker, "html", "set_html", set_html)?;
    register_wasm_function!(linker, "html", "prepend", prepend)?;
    register_wasm_function!(linker, "html", "append", append)?;
    register_wasm_function!(linker, "html", "first", first)?;
    register_wasm_function!(linker, "html", "last", last)?;
    register_wasm_function!(linker, "html", "next", next)?;
    register_wasm_function!(linker, "html", "previous", previous)?;

    register_wasm_function!(linker, "html", "base_uri", base_uri)?;
    register_wasm_function!(linker, "html", "body", body)?;
    register_wasm_function!(linker, "html", "text", text)?;
    register_wasm_function!(linker, "html", "untrimmed_text", untrimmed_text)?;
    register_wasm_function!(linker, "html", "own_text", own_text)?;

    register_wasm_function!(linker, "html", "data", data)?;
    register_wasm_function!(linker, "html", "array", array)?;
    register_wasm_function!(linker, "html", "html", html)?;
    register_wasm_function!(linker, "html", "outer_html", outer_html)?;

    register_wasm_function!(linker, "html", "escape", escape)?;
    register_wasm_function!(linker, "html", "unescape", unescape)?;
    register_wasm_function!(linker, "html", "id", id)?;
    register_wasm_function!(linker, "html", "tag_name", tag_name)?;
    register_wasm_function!(linker, "html", "class_name", class_name)?;
    register_wasm_function!(linker, "html", "has_class", has_class)?;
    register_wasm_function!(linker, "html", "has_attr", has_attr)?;

    Ok(())
}

#[aidoku_wasm_function]
fn parse(caller: Caller<'_, WasmStore>, data: Option<String>) -> i32 {
    parse_with_uri(caller, data, None)
}

#[aidoku_wasm_function]
fn parse_fragment(caller: Caller<'_, WasmStore>, data: Option<String>) -> i32 {
    parse_fragment_with_uri(caller, data, None)
}

#[aidoku_wasm_function]
fn parse_with_uri(
    mut caller: Caller<'_, WasmStore>,
    data: Option<String>,
    uri: Option<String>,
) -> i32 {
    || -> Option<i32> {
        let document = ScraperHtml::parse_document(&data?);
        let node_id = document.root_element().id();
        let uri = match uri {
            Some(uri) => Some(Url::parse(&uri).ok()?),
            None => None,
        };
        let html_element = HTMLElement {
            document: Html::from(document).into(),
            node_id,
            base_uri: uri,
        };

        let wasm_store = caller.data_mut();

        Some(wasm_store.store_std_value(Value::from(vec![html_element]).into(), None) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn parse_fragment_with_uri(
    mut caller: Caller<'_, WasmStore>,
    data: Option<String>,
    uri: Option<String>,
) -> i32 {
    || -> Option<i32> {
        let fragment = ScraperHtml::parse_fragment(&data?);
        let node_id = fragment.root_element().id();
        let uri = match uri {
            Some(uri) => Some(Url::parse(&uri).ok()?),
            None => None,
        };
        let html_element = HTMLElement {
            document: Html::from(fragment).into(),
            node_id,
            base_uri: uri,
        };

        let wasm_store = caller.data_mut();

        Some(wasm_store.store_std_value(Value::from(vec![html_element]).into(), None) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn select(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32, selector: Option<String>) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;
        let mut selector_str = selector?;
        
        // INFO: WeebCentral Workaround
        if selector_str == "section[x-data~=scroll] > img" {
            selector_str = "img[src]".to_string();
        }
        
        let wasm_store = caller.data_mut();
        let std_value = wasm_store.get_std_value(descriptor)?;
        let html_elements = match std_value.as_ref() {
            Value::HTMLElements(elements) => Some(elements),
            _ => None,
        }?;

        let selector = Selector::parse(&selector_str).ok()?;
        let selected_elements: Vec<_> = html_elements
            .iter()
            .flat_map(|element| {
                element
                    .element_ref()
                    .select(&selector)
                    .map(|selected_element_ref| HTMLElement {
                        document: element.document.clone(),
                        node_id: selected_element_ref.id(),
                        base_uri: element.base_uri.clone(),
                    })
            })
            .collect();

        Some(
            wasm_store.store_std_value(Value::from(selected_elements).into(), Some(descriptor))
                as i32,
        )
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn attr(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32, selector: Option<String>) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;
        let selector = selector?;

        // SwiftSoup uses "abs:" as a prefix for attributes that should be converted to
        // absolute URLs (e.g. href, src). Some Aidoku extensions actually use this (for some stupid
        // reason), so we need to support it.
        let has_abs_prefix = selector.starts_with("abs:");
        let selector = if has_abs_prefix {
            selector.strip_prefix("abs:").unwrap().to_string()
        } else {
            selector
        };

        let wasm_store = caller.data_mut();
        let std_value = wasm_store.get_std_value(descriptor)?;
        let elements = match std_value.as_ref() {
            Value::HTMLElements(elements) => Some(elements),
            _ => None,
        }?;

        let attr = elements
            .iter()
            .map(|element| {
                let element_value = element.element_ref().value();

                element_value.attr(&selector)
            })
            .find(|element| element.is_some())?
            .unwrap()
            .to_string();

        let attr = if has_abs_prefix {
            let base_uri = elements
                .iter()
                .find_map(|element| element.base_uri.as_ref())?;
            let attr_url = Url::parse(&attr).ok()?;
            let absolute_url = base_uri.join(attr_url.as_str()).ok()?;

            absolute_url.to_string()
        } else {
            attr
        };

        Some(wasm_store.store_std_value(Value::from(attr).into(), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn set_text(mut _caller: Caller<'_, WasmStore>, descriptor_i32: i32, text: Option<String>) -> i32 {
    || -> Option<i32> {
        let _descriptor: usize = descriptor_i32.try_into().ok()?;
        let _text = text?;

        todo!("modifying the HTML document is unsupported")
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn set_html(_caller: Caller<'_, WasmStore>, _descriptor_i32: i32, _text: Option<String>) -> i32 {
    todo!("modifying the HTML document is unsupported")
}

#[aidoku_wasm_function]
fn prepend(_caller: Caller<'_, WasmStore>, _descriptor_i32: i32, _text: Option<String>) -> i32 {
    todo!("modifying the HTML document is unsupported")
}

#[aidoku_wasm_function]
fn append(_caller: Caller<'_, WasmStore>, _descriptor_i32: i32, _text: Option<String>) -> i32 {
    todo!("modifying the HTML document is unsupported")
}

#[aidoku_wasm_function]
fn first(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let std_value = wasm_store.get_std_value(descriptor)?;
        // TODO should this work if we have only one element? i guess so..?
        let element = match std_value.as_ref() {
            Value::HTMLElements(elements) => Some(elements.first().unwrap().clone()),
            _ => None,
        }?;

        Some(wasm_store.store_std_value(Value::from(vec![element]).into(), None) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn last(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        // TODO should this work if we have only one element? i guess so..?
        let element = match wasm_store.get_std_value(descriptor)?.as_ref() {
            Value::HTMLElements(elements) => Some(elements.last().unwrap().clone()),
            _ => None,
        }?;

        Some(wasm_store.store_std_value(Value::from(vec![element]).into(), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn next(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let std_value = wasm_store.get_std_value(descriptor)?;
        let element = match std_value.as_ref() {
            Value::HTMLElements(elements) if elements.len() == 1 => {
                Some(elements.last().unwrap().clone())
            }
            _ => None,
        }?;

        let next_sibling_node_id = element.element_ref().next_sibling_element()?.id();
        let new_element = HTMLElement {
            document: element.document.clone(),
            node_id: next_sibling_node_id,
            base_uri: element.base_uri.clone(),
        };

        Some(
            wasm_store.store_std_value(Value::from(vec![new_element]).into(), Some(descriptor))
                as i32,
        )
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn previous(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let element = match wasm_store.get_std_value(descriptor)?.as_ref() {
            Value::HTMLElements(elements) if elements.len() == 1 => {
                Some(elements.last().unwrap().clone())
            }
            _ => None,
        }?;

        let next_sibling_node_id = element.element_ref().next_sibling_element()?.id();
        let new_element = HTMLElement {
            document: element.document.clone(),
            node_id: next_sibling_node_id,
            base_uri: element.base_uri.clone(),
        };

        Some(
            wasm_store.store_std_value(Value::from(vec![new_element]).into(), Some(descriptor))
                as i32,
        )
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn base_uri(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let element = match wasm_store.get_std_value(descriptor)?.as_ref() {
            Value::HTMLElements(elements) if elements.len() == 1 => {
                Some(elements.last().unwrap().clone())
            }
            _ => None,
        }?;

        let base_uri = element.base_uri?.to_string();

        Some(wasm_store.store_std_value(Value::from(base_uri).into(), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn body(caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    select(caller, descriptor_i32, Some("body".into()))
}

#[aidoku_wasm_function]
fn text(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let std_value = wasm_store.get_std_value(descriptor)?;
        let elements = match std_value.as_ref() {
            Value::HTMLElements(elements) => Some(elements),
            _ => None,
        }?;

        let text = elements
            .iter()
            .flat_map(|element| element.element_ref().text())
            .map(|s| s.trim())
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();

        Some(wasm_store.store_std_value(Value::from(text).into(), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn untrimmed_text(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let std_value = wasm_store.get_std_value(descriptor)?;
        let elements = match std_value.as_ref() {
            Value::HTMLElements(elements) => Some(elements),
            _ => None,
        }?;

        let text = elements
            .iter()
            .flat_map(|element| element.element_ref().text())
            .collect::<Vec<_>>()
            .join(" ");

        Some(wasm_store.store_std_value(Value::from(text).into(), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn own_text(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let std_value = wasm_store.get_std_value(descriptor)?;
        let own_text = match std_value.as_ref() {
            Value::HTMLElements(elements) if elements.len() == 1 => {
                let element = elements.first().unwrap();
                let own_text = element
                    .element_ref()
                    .children()
                    .filter_map(|node_ref| match node_ref.value() {
                        // FIXME WHAT
                        Node::Text(text) => Some(&**text),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");

                Some(own_text)
            }
            Value::String(s) => Some(s.to_string()), // what the fuck why is this valid i dont fucking know
            _ => None,
        }?;

        Some(wasm_store.store_std_value(Value::String(own_text).into(), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn data(_caller: Caller<'_, WasmStore>, _descriptor_i32: i32) -> i32 {
    todo!("yeah idk man")
}

#[aidoku_wasm_function]
fn array(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let std_value = wasm_store.get_std_value(descriptor)?;
        let elements = match std_value.as_ref() {
            // why
            Value::HTMLElements(elements) => Some(elements),
            _ => None,
        }?;

        let array_value: Vec<Value> = elements
            .iter()
            .map(|element| vec![element.clone()].into())
            .collect();

        Some(wasm_store.store_std_value(Value::from(array_value).into(), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn html(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let std_value = wasm_store.get_std_value(descriptor)?;
        let elements = match std_value.as_ref() {
            Value::HTMLElements(elements) => Some(elements),
            _ => None,
        }?;

        let inner_htmls = elements
            .iter()
            .map(|element| element.element_ref().inner_html())
            .collect::<Vec<_>>()
            .join("\n");

        Some(wasm_store.store_std_value(Value::from(inner_htmls).into(), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn outer_html(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let std_value = wasm_store.get_std_value(descriptor)?;
        let elements = match std_value.as_ref() {
            Value::HTMLElements(elements) => Some(elements),
            _ => None,
        }?;

        let htmls = elements
            .iter()
            .map(|element| element.element_ref().html())
            .collect::<Vec<_>>()
            .join("\n");

        Some(wasm_store.store_std_value(Value::from(htmls).into(), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn escape(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let text = match wasm_store.get_std_value(descriptor)?.as_ref() {
            Value::HTMLElements(elements) => {
                let text = elements
                    .iter()
                    .flat_map(|element| element.element_ref().text())
                    .map(|s| s.trim())
                    .collect::<Vec<_>>()
                    .join(" ");

                Some(text)
            }
            Value::String(s) => Some(s.to_owned()),
            _ => None,
        }?;

        let escaped = html_escape::encode_safe(&text).to_string();

        Some(wasm_store.store_std_value(Value::from(escaped).into(), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn unescape(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let text = match wasm_store.get_std_value(descriptor)?.as_ref() {
            Value::HTMLElements(elements) => {
                let text = elements
                    .iter()
                    .flat_map(|element| element.element_ref().text())
                    .map(|s| s.trim())
                    .collect::<Vec<_>>()
                    .join(" ");

                Some(text)
            }
            Value::String(s) => Some(s.to_owned()),
            _ => None,
        }?;

        let unescaped = html_escape::decode_html_entities(&text).to_string();

        Some(wasm_store.store_std_value(Value::from(unescaped).into(), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn id(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let element = match wasm_store.get_std_value(descriptor)?.as_ref() {
            Value::HTMLElements(elements) if elements.len() == 1 => {
                Some(elements.last().unwrap().clone())
            }
            _ => None,
        }?;

        let id = element.element_ref().value().id()?.to_string();

        Some(wasm_store.store_std_value(Value::from(id).into(), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn tag_name(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let element = match wasm_store.get_std_value(descriptor)?.as_ref() {
            Value::HTMLElements(elements) if elements.len() == 1 => {
                Some(elements.last().unwrap().clone())
            }
            _ => None,
        }?;

        let tag_name = element.element_ref().value().name().to_string();

        Some(wasm_store.store_std_value(Value::from(tag_name).into(), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn class_name(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let element = match wasm_store.get_std_value(descriptor)?.as_ref() {
            Value::HTMLElements(elements) if elements.len() == 1 => {
                Some(elements.last().unwrap().clone())
            }
            _ => None,
        }?;

        let class_name = element
            .element_ref()
            .value()
            .attr("class")?
            .trim()
            .to_string();

        Some(wasm_store.store_std_value(Value::from(class_name).into(), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn has_class(
    mut caller: Caller<'_, WasmStore>,
    descriptor_i32: i32,
    class_name: Option<String>,
) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;
        let class_name = class_name?;

        let wasm_store = caller.data_mut();
        let element = match wasm_store.get_std_value(descriptor)?.as_ref() {
            Value::HTMLElements(elements) if elements.len() == 1 => {
                Some(elements.last().unwrap().clone())
            }
            _ => None,
        }?;

        let has_class = element
            .element_ref()
            .value()
            .classes()
            .any(|class| class == class_name);

        Some(if has_class { 1 } else { 0 })
    }()
    .unwrap_or(0)
}

#[aidoku_wasm_function]
fn has_attr(
    mut caller: Caller<'_, WasmStore>,
    descriptor_i32: i32,
    attr_name: Option<String>,
) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;
        let attr_name = attr_name?;

        let wasm_store = caller.data_mut();
        let element = match wasm_store.get_std_value(descriptor)?.as_ref() {
            Value::HTMLElements(elements) if elements.len() == 1 => {
                Some(elements.last().unwrap().clone())
            }
            _ => None,
        }?;

        let has_attr = element
            .element_ref()
            .value()
            .attrs()
            .any(|(name, _)| name == attr_name);

        Some(if has_attr { 1 } else { 0 })
    }()
    .unwrap_or(0)
}
