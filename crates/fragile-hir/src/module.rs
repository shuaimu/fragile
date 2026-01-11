use fragile_common::{SourceId, Symbol};
use crate::item::Item;

/// A module in the HIR (corresponds to a source file or module declaration).
#[derive(Debug, Clone)]
pub struct Module {
    pub name: Symbol,
    pub source: SourceId,
    pub items: Vec<Item>,
}

impl Module {
    pub fn new(name: Symbol, source: SourceId) -> Self {
        Self {
            name,
            source,
            items: Vec::new(),
        }
    }

    pub fn add_item(&mut self, item: Item) {
        self.items.push(item);
    }
}

/// The entire program representation.
#[derive(Debug, Default)]
pub struct Program {
    pub modules: Vec<Module>,
}

impl Program {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_module(&mut self, module: Module) {
        self.modules.push(module);
    }
}
