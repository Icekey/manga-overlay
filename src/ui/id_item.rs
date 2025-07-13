use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct IdItem<T> {
    id: usize,
    pub item: T,
    pub active: bool,
}

impl<T> IdItem<T> {
    pub fn from_vec(items: Vec<T>) -> Vec<IdItem<T>> {
        items
            .into_iter()
            .enumerate()
            .map(|(id, item)| IdItem {
                id,
                item,
                active: true,
            })
            .collect()
    }
}

impl<T> Hash for IdItem<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.id);
    }
}

pub trait IdItemVec<T>
where
    T: Clone,
{
    fn push_item(&mut self, item: T);

    fn create_active_item_vec(&self) -> Vec<T>;
}

impl<T> IdItemVec<T> for Vec<IdItem<T>>
where
    T: Clone,
{
    fn push_item(&mut self, item: T) {
        let id = self
            .iter()
            .map(|i| i.id)
            .max()
            .map(|x| x + 1)
            .unwrap_or_default();

        self.push(IdItem {
            id,
            item,
            active: true,
        });
    }

    fn create_active_item_vec(&self) -> Vec<T> {
        self.iter()
            .filter(|x| x.active)
            .map(|i| i.item.clone())
            .collect()
    }
}
