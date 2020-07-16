use crate::core::store::{FromKey, Index, LoadHandler, OnLoad, ReadGuard, WriteGuard};

/// Generalized id that can store a key or an index. The key is turned into a index
/// during the get (get_mut) opertaion
/// get (get_mut) operation the id is turned into index.
pub enum AutoNamedId<D>
where
    D: FromKey,
{
    Name(D::Key),
    Index(Index<D>),
}

impl<D> AutoNamedId<D>
where
    D: FromKey,
{
    pub fn from_key(key: D::Key) -> Self {
        AutoNamedId::Name(key)
    }
}

impl<L, D> AutoNamedId<D>
where
    D: FromKey + OnLoad<LoadHandler = L>,
    L: 'static + LoadHandler<D>,
{
    pub fn get<'a, 's>(&'a mut self, store: &'a mut ReadGuard<'s, D, L>) -> &'a D {
        if let AutoNamedId::Name(name) = self {
            let idx = store.get_or_load(name);
            *self = AutoNamedId::Index(idx);
        }

        if let AutoNamedId::Index(idx) = self {
            store.at(idx)
        } else {
            unreachable!()
        }
    }

    pub fn get_mut<'a>(&'a mut self, store: &'a mut WriteGuard<'a, D, L>) -> &'a mut D
    where
        D: OnLoad<LoadHandler = L>,
        L: 'static + LoadHandler<D>,
    {
        if let AutoNamedId::Name(name) = self {
            let idx = store.get_or_load(name);
            *self = AutoNamedId::Index(idx);
        }

        if let AutoNamedId::Index(idx) = self {
            store.at_mut(idx)
        } else {
            unreachable!()
        }
    }
}

impl<D> Clone for AutoNamedId<D>
where
    D: FromKey,
{
    fn clone(&self) -> AutoNamedId<D> {
        match self {
            AutoNamedId::Index(idx) => AutoNamedId::Index(idx.clone()),
            AutoNamedId::Name(name) => AutoNamedId::Name(name.clone()),
        }
    }
}
