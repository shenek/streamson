use lazy_static::lazy_static;
use std::collections::HashMap;

use super::Element;
use crate::create_doc_element;

create_doc_element!(Simple, "simple", &["simple", "s"], Some("DEFINITION"), "");

lazy_static! {
    pub static ref MAP: HashMap<&'static str, &'static dyn Element> = {
        let mut res: HashMap<&'static str, &'static dyn Element> = HashMap::new();
        res.insert(Simple.as_ref(), &Simple as &dyn Element);
        res
    };
}
