use std::sync::Arc;

use anyhow::Result;
use scraper::{Element, Html, Node, Selector};
use wasm_macros::{aidoku_wasm_function, register_wasm_function};
use wasmi::{Caller, Linker};

use crate::source::wasm_store::{HTMLElement, Value, WasmStore};

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
fn parse(mut caller: Caller<'_, WasmStore>, data: Option<String>) -> i32 {
    || -> Option<i32> {
        let document = Arc::new(Html::parse_document(&data?));
        let node_id = document.root_element().id();
        let html_element = HTMLElement { document, node_id };
        let wasm_store = caller.data_mut();

        Some(wasm_store.store_std_value(Value::HTMLElements(vec![html_element]), None) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn parse_fragment(mut caller: Caller<'_, WasmStore>, data: Option<String>) -> i32 {
    || -> Option<i32> {
        let fragment = Arc::new(Html::parse_fragment(&data?));
        let node_id = fragment.root_element().id();
        let html_element = HTMLElement {
            document: fragment,
            node_id,
        };

        let wasm_store = caller.data_mut();

        Some(wasm_store.store_std_value(Value::HTMLElements(vec![html_element]), None) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn parse_with_uri(
    caller: Caller<'_, WasmStore>,
    data: Option<String>,
    _uri: Option<String>,
) -> i32 {
    // TODO tratar uri :thumbsup:
    parse(caller, data)
}

#[aidoku_wasm_function]
fn parse_fragment_with_uri(
    caller: Caller<'_, WasmStore>,
    data: Option<String>,
    _uri: Option<String>,
) -> i32 {
    // TODO tratar uri :thumbsup:
    parse_fragment(caller, data)
}

#[aidoku_wasm_function]
fn select(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32, selector: Option<String>) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;
        let wasm_store = caller.data_mut();
        let html_elements = match wasm_store.read_std_value(descriptor)? {
            Value::HTMLElements(elements) => Some(elements),
            _ => None,
        }?;

        // TODO NAMING IS PURE GARBAGE
        let selector = Selector::parse(&selector?).ok()?;
        let selected_elements: Vec<_> = html_elements
            .iter()
            .flat_map(|element| {
                element
                    .element_ref()
                    .select(&selector)
                    .map(|selected_element_ref| HTMLElement {
                        document: element.document.clone(),
                        node_id: selected_element_ref.id(),
                    })
            })
            .collect();

        Some(
            wasm_store.store_std_value(Value::HTMLElements(selected_elements), Some(descriptor))
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

        let wasm_store = caller.data_mut();
        let elements = match wasm_store.read_std_value(descriptor)? {
            Value::HTMLElements(elements) => Some(elements),
            _ => None,
        }?;

        let attr = elements
            .iter()
            .map(|element| element.element_ref().value().attr(&selector))
            .find(|element| element.is_some())?
            .unwrap();

        Some(wasm_store.store_std_value(Value::String(attr.into()), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn set_text(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32, text: Option<String>) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;
        let _text = text?;

        let wasm_store = caller.data_mut();
        let _element = match wasm_store.get_mut_std_value(descriptor)? {
            Value::HTMLElements(elements) if elements.len() == 1 => {
                Some(elements.first_mut().unwrap())
            }
            _ => None,
        }?;

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
        // TODO should this work if we have only one element? i guess so..?
        let element = match wasm_store.read_std_value(descriptor)? {
            Value::HTMLElements(elements) => Some(elements.first().unwrap().clone()),
            _ => None,
        }?;

        Some(
            wasm_store.store_std_value(Value::HTMLElements(vec![element]), Some(descriptor)) as i32,
        )
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn last(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        // TODO should this work if we have only one element? i guess so..?
        let element = match wasm_store.read_std_value(descriptor)? {
            Value::HTMLElements(elements) => Some(elements.last().unwrap().clone()),
            _ => None,
        }?;

        Some(
            wasm_store.store_std_value(Value::HTMLElements(vec![element]), Some(descriptor)) as i32,
        )
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn next(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let element = match wasm_store.read_std_value(descriptor)? {
            Value::HTMLElements(elements) if elements.len() == 1 => {
                Some(elements.last().unwrap().clone())
            }
            _ => None,
        }?;

        let next_sibling_node_id = element.element_ref().next_sibling_element()?.id();
        let new_element = HTMLElement {
            document: element.document.clone(),
            node_id: next_sibling_node_id,
        };

        Some(
            wasm_store.store_std_value(Value::HTMLElements(vec![new_element]), Some(descriptor))
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
        let element = match wasm_store.read_std_value(descriptor)? {
            Value::HTMLElements(elements) if elements.len() == 1 => {
                Some(elements.last().unwrap().clone())
            }
            _ => None,
        }?;

        let next_sibling_node_id = element.element_ref().next_sibling_element()?.id();
        let new_element = HTMLElement {
            document: element.document.clone(),
            node_id: next_sibling_node_id,
        };

        Some(
            wasm_store.store_std_value(Value::HTMLElements(vec![new_element]), Some(descriptor))
                as i32,
        )
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn base_uri(_caller: Caller<'_, WasmStore>, _descriptor_i32: i32) -> i32 {
    todo!("wtf")
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
        let elements = match wasm_store.read_std_value(descriptor)? {
            Value::HTMLElements(elements) => Some(elements),
            _ => None,
        }?;

        let text = elements
            .iter()
            .flat_map(|element| element.element_ref().text())
            .map(|s| s.trim())
            .collect::<Vec<_>>()
            .join(" ");

        Some(wasm_store.store_std_value(Value::String(text), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn untrimmed_text(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let elements = match wasm_store.read_std_value(descriptor)? {
            Value::HTMLElements(elements) => Some(elements),
            _ => None,
        }?;

        let text = elements
            .iter()
            .flat_map(|element| element.element_ref().text())
            .collect::<Vec<_>>()
            .join(" ");

        Some(wasm_store.store_std_value(Value::String(text), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn own_text(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let own_text = match wasm_store.read_std_value(descriptor)? {
            Value::HTMLElements(elements) if elements.len() == 1 => {
                let element = elements.first().unwrap();
                let own_text = element
                    .element_ref()
                    .children()
                    .filter_map(|node_ref| match node_ref.value() {
                        Node::Text(text) => Some(text.to_string()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");

                Some(own_text)
            }
            Value::String(s) => Some(s), // what the fuck why is this valid i dont fucking know
            _ => None,
        }?;

        Some(wasm_store.store_std_value(Value::String(own_text), Some(descriptor)) as i32)
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
        let elements = match wasm_store.read_std_value(descriptor)? {
            // why
            Value::HTMLElements(elements) => Some(elements),
            _ => None,
        }?;

        let array_value: Vec<_> = elements
            .iter()
            .map(|element| Value::HTMLElements(vec![element.clone()]))
            .collect();

        Some(wasm_store.store_std_value(Value::Array(array_value), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn html(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let elements = match wasm_store.read_std_value(descriptor)? {
            Value::HTMLElements(elements) => Some(elements),
            _ => None,
        }?;

        let inner_htmls = elements
            .iter()
            .map(|element| element.element_ref().inner_html())
            .collect::<Vec<_>>()
            .join("\n");

        Some(wasm_store.store_std_value(Value::String(inner_htmls), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn outer_html(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let elements = match wasm_store.read_std_value(descriptor)? {
            Value::HTMLElements(elements) => Some(elements),
            _ => None,
        }?;

        let htmls = elements
            .iter()
            .map(|element| element.element_ref().html())
            .collect::<Vec<_>>()
            .join("\n");

        Some(wasm_store.store_std_value(Value::String(htmls), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn escape(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let text = match wasm_store.read_std_value(descriptor)? {
            Value::HTMLElements(elements) => {
                let text = elements
                    .iter()
                    .flat_map(|element| element.element_ref().text())
                    .map(|s| s.trim())
                    .collect::<Vec<_>>()
                    .join(" ");

                Some(text)
            }
            Value::String(s) => Some(s),
            _ => None,
        }?;

        let escaped = html_escape::encode_safe(&text).to_string();

        Some(wasm_store.store_std_value(Value::String(escaped), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn unescape(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let text = match wasm_store.read_std_value(descriptor)? {
            Value::HTMLElements(elements) => {
                let text = elements
                    .iter()
                    .flat_map(|element| element.element_ref().text())
                    .map(|s| s.trim())
                    .collect::<Vec<_>>()
                    .join(" ");

                Some(text)
            }
            Value::String(s) => Some(s),
            _ => None,
        }?;

        let unescaped = html_escape::decode_html_entities(&text).to_string();

        Some(wasm_store.store_std_value(Value::String(unescaped), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn id(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let element = match wasm_store.read_std_value(descriptor)? {
            Value::HTMLElements(elements) if elements.len() == 1 => {
                Some(elements.last().unwrap().clone())
            }
            _ => None,
        }?;

        let id = element.element_ref().value().id()?.to_string();

        Some(wasm_store.store_std_value(Value::String(id), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn tag_name(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let element = match wasm_store.read_std_value(descriptor)? {
            Value::HTMLElements(elements) if elements.len() == 1 => {
                Some(elements.last().unwrap().clone())
            }
            _ => None,
        }?;

        let tag_name = element.element_ref().value().name().to_string();

        Some(wasm_store.store_std_value(Value::String(tag_name), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn class_name(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let element = match wasm_store.read_std_value(descriptor)? {
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

        Some(wasm_store.store_std_value(Value::String(class_name), Some(descriptor)) as i32)
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
        let element = match wasm_store.read_std_value(descriptor)? {
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
        let element = match wasm_store.read_std_value(descriptor)? {
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
