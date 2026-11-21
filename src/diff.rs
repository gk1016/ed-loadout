//! `diff`: compare two profiles. Element-level for the flat graphics XML,
//! unified line diff for everything else.

use crate::bundle::{join_rel, scan_profile};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// Files compared element-by-element (flat child elements of the root).
pub fn is_flat_xml(rel: &str) -> bool {
    match rel.split_once('/') {
        Some(("Graphics", name)) => {
            (name.starts_with("Custom.") && name.ends_with(".fxcfg"))
                || name == "Settings.xml"
                || name == "DisplaySettings.xml"
        }
        _ => false,
    }
}

pub fn run(a: &Path, b: &Path) -> Result<i32, String> {
    let fa: BTreeSet<String> = scan_profile(a)?.into_iter().collect();
    let fb: BTreeSet<String> = scan_profile(b)?.into_iter().collect();
    let mut differ = false;

    for rel in fa.difference(&fb) {
        println!("only in a: {rel}");
        differ = true;
    }
    for rel in fb.difference(&fa) {
        println!("only in b: {rel}");
        differ = true;
    }
    for rel in fa.intersection(&fb) {
        let pa = join_rel(&a.join("files"), rel);
        let pb = join_rel(&b.join("files"), rel);
        let ba = fs::read(&pa).map_err(|e| format!("{}: {e}", pa.display()))?;
        let bb = fs::read(&pb).map_err(|e| format!("{}: {e}", pb.display()))?;
        if ba == bb {
            continue;
        }
        differ = true;
        let ta = String::from_utf8_lossy(&ba);
        let tb = String::from_utf8_lossy(&bb);
        if is_flat_xml(rel) {
            if let (Some(ea), Some(eb)) = (flat_elements(&ta), flat_elements(&tb)) {
                let lines = element_diff(&ea, &eb);
                if lines.is_empty() {
                    println!("{rel}: differs outside element values");
                }
                for (name, old, new) in lines {
                    println!("{rel}: {name} {old} -> {new}");
                }
                continue;
            }
            eprintln!("note: {rel} is not well-formed XML; showing a line diff");
        }
        print!(
            "{}",
            similar::TextDiff::from_lines(ta.as_ref(), tb.as_ref())
                .unified_diff()
                .header(&format!("a/{rel}"), &format!("b/{rel}"))
        );
    }
    Ok(if differ { 1 } else { 0 })
}

/// (name, text) for each child element of the root, in document order.
fn flat_elements(text: &str) -> Option<Vec<(String, String)>> {
    let doc = roxmltree::Document::parse(text).ok()?;
    Some(
        doc.root_element()
            .children()
            .filter(|n| n.is_element())
            .map(|n| {
                (
                    n.tag_name().name().to_owned(),
                    n.text().unwrap_or("").trim().to_owned(),
                )
            })
            .collect(),
    )
}

fn element_diff(a: &[(String, String)], b: &[(String, String)]) -> Vec<(String, String, String)> {
    const ABSENT: &str = "(absent)";
    let lookup = |v: &[(String, String)], name: &str| {
        v.iter().find(|(n, _)| n == name).map(|(_, t)| t.clone())
    };
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for (name, _) in a.iter().chain(b.iter()) {
        if !seen.insert(name.clone()) {
            continue;
        }
        let old = lookup(a, name);
        let new = lookup(b, name);
        if old != new {
            out.push((
                name.clone(),
                old.unwrap_or_else(|| ABSENT.into()),
                new.unwrap_or_else(|| ABSENT.into()),
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_diff_reports_change_add_remove() {
        let a = vec![("A".into(), "1".into()), ("B".into(), "2".into())];
        let b = vec![("A".into(), "1".into()), ("C".into(), "3".into())];
        let d = element_diff(&a, &b);
        assert_eq!(
            d,
            vec![
                ("B".to_string(), "2".to_string(), "(absent)".to_string()),
                ("C".to_string(), "(absent)".to_string(), "3".to_string()),
            ]
        );
    }
}
