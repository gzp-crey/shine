use crate::crdt::Id;

pub struct ItemId(usize);

impl From<usize> for ItemId {
    fn from(c: usize) -> ItemId {
        ItemId(c)
    }
}

pub struct Item {
    /// Unique Id of this item
    id: Id,

    /// The item that is currently to the left of this item.
    left: Option<ItemId>,

    /// The item that was originally to the left of this item.
    left_origin: Option<Id>,

    /// The item that is currently to the right of this item.
    right: Option<ItemId>,

    /// The item that was originally to the right of this item.
    right_origin: Option<Id>,
    //parent
    // If this type's effect is reundone this type refers to the type that undid this operation.
    //redone = Option<Id>
    //content = Option<ContentId>
}

pub struct ItemList {
    items: Vec<Item>,
}

impl ItemList {
    pub fn new() -> ItemList {
        ItemList { items: Vec::new() }
    }
}
