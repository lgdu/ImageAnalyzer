use crate::types::MetadataEntry;
use quick_xml::events::Event;
use quick_xml::Reader as XmlReader;

pub fn read_xmp(data: &[u8]) -> Result<Vec<MetadataEntry>, String> {
    let xml_str = String::from_utf8_lossy(data);

    let mut reader = XmlReader::from_str(&xml_str);
    reader.config_mut().trim_text(true);

    let mut entries = Vec::new();
    let mut buf = Vec::new();

    // First pass: extract ALL attributes from rdf:Description (not just whitelisted ones)
    // and text from known elements
    let mut current_element: Option<String> = None;
    let mut text_buffer = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if name == "rdf:Description" {
                    // Extract ALL attributes from rdf:Description, not just whitelisted
                    for attr in e.attributes().flatten() {
                        let attr_name = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        let attr_value = String::from_utf8_lossy(&attr.value)
                            .to_string()
                            .trim()
                            .to_string();

                        // Skip xmlns declarations and empty values
                        if attr_name.starts_with("xmlns") || attr_value.is_empty() {
                            continue;
                        }

                        let display_name = map_xmp_attr_name(&attr_name);
                        if !display_name.is_empty() {
                            entries.push(MetadataEntry {
                                standard: "XMP".to_string(),
                                tag_name: display_name,
                                tag_value: attr_value.clone(),
                                raw_value: Some(attr_value),
                            });
                        }
                    }
                } else {
                    // For child elements of rdf:Description, also extract their attributes
                    // (e.g., <xmpMM:DerivedFrom stRef:instanceID="...">)
                    if current_element.is_some() {
                        // We're inside rdf:Description - extract attributes from child elements
                        for attr in e.attributes().flatten() {
                            let attr_name = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let attr_value = String::from_utf8_lossy(&attr.value)
                                .to_string()
                                .trim()
                                .to_string();

                            if attr_name.starts_with("xmlns") || attr_value.is_empty() {
                                continue;
                            }

                            let display_name = map_xmp_attr_name(&attr_name);
                            if !display_name.is_empty() {
                                entries.push(MetadataEntry {
                                    standard: "XMP".to_string(),
                                    tag_name: display_name,
                                    tag_value: attr_value.clone(),
                                    raw_value: Some(attr_value),
                                });
                            }
                        }
                    }

                    current_element = Some(name);
                    text_buffer.clear();
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = String::from_utf8_lossy(e.as_ref())
                    .to_string()
                    .trim()
                    .to_string();
                text_buffer = text;
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if name == "rdf:Description" {
                    // Exiting rdf:Description - reset tracking
                    current_element = None;
                } else {
                    // If this element had text content and is a known XMP field
                    if let Some(ref elem_name) = current_element {
                        if elem_name == &name && !text_buffer.is_empty() {
                            let display_name = map_xmp_elem_name(elem_name);
                            let tag_name = if display_name.is_empty() {
                                map_xmp_attr_name(elem_name)
                            } else {
                                display_name
                            };
                            if !tag_name.is_empty() {
                                entries.push(MetadataEntry {
                                    standard: "XMP".to_string(),
                                    tag_name,
                                    tag_value: text_buffer.clone(),
                                    raw_value: Some(text_buffer.clone()),
                                });
                            }
                        }
                    }
                    current_element = None;
                }
            }
            Err(e) => {
                if entries.is_empty() {
                    return Err(format!("Failed to parse XMP XML: {}", e));
                }
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    // Also extract from <rdf:Bag> and similar structures
    let mut bag_entries = extract_bag_li_items(&xml_str);
    entries.append(&mut bag_entries);

    if entries.is_empty() {
        return Err("No recognizable XMP metadata found".to_string());
    }

    Ok(entries)
}

fn map_xmp_attr_name(name: &str) -> String {
    // Map qualified names like "xmp:CreatorTool" to "CreatorTool"
    if let Some(colon_pos) = name.find(':') {
        name[colon_pos + 1..].to_string()
    } else {
        name.to_string()
    }
}

fn map_xmp_elem_name(name: &str) -> String {
    // Map element names to display names
    match name {
        "rdf:li" => return String::new(), // handled by bag extraction
        "dc:format" => return "Format".to_string(),
        "dc:description" => return "Description".to_string(),
        "dc:title" => return "Title".to_string(),
        "dc:creator" => return "Creator".to_string(),
        "dc:rights" => return "Rights".to_string(),
        "dc:subject" => return "Subject".to_string(),
        "dc:publisher" => return "Publisher".to_string(),
        "dc:contributor" => return "Contributor".to_string(),
        "dc:coverage" => return "Coverage".to_string(),
        "dc:identifier" => return "Identifier".to_string(),
        "dc:language" => return "Language".to_string(),
        "dc:source" => return "Source".to_string(),
        _ => {}
    }
    map_xmp_attr_name(name)
}

/// Extract list items from rdf:Bag, rdf:Seq, rdf:Alt structures
fn extract_bag_li_items(xml: &str) -> Vec<MetadataEntry> {
    let mut entries = Vec::new();
    let mut reader = XmlReader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut in_bag_or_seq = false;
    let mut parent_name = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "rdf:Bag" || name == "rdf:Seq" || name == "rdf:Alt" {
                    in_bag_or_seq = true;
                    // The parent element name becomes the tag
                    parent_name = name;
                } else if in_bag_or_seq && name == "rdf:li" {
                    // Next event should be text
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_bag_or_seq {
                    let text = String::from_utf8_lossy(e.as_ref())
                        .to_string()
                        .trim()
                        .to_string();
                    if !text.is_empty() {
                        entries.push(MetadataEntry {
                            standard: "XMP".to_string(),
                            tag_name: format!("{} item", parent_name),
                            tag_value: text.clone(),
                            raw_value: Some(text),
                        });
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "rdf:Bag" || name == "rdf:Seq" || name == "rdf:Alt" {
                    in_bag_or_seq = false;
                }
            }
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    entries
}
