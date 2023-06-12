mod de_bruijn;
pub mod expr;

pub use de_bruijn::*;

use upcast::{Upcast, UpcastFrom};

#[salsa::jar(db = Db)]
pub struct Jar(expr::Expression);

pub trait Db: files::Db + Upcast<dyn files::Db> + salsa::DbWithJar<Jar> {}

impl<T> Db for T where T: files::Db + salsa::DbWithJar<Jar> + 'static {}

impl<'a, T: Db + 'a> UpcastFrom<T> for dyn Db + 'a {
    fn up_from(value: &T) -> &(dyn Db + 'a) {
        value
    }
    fn up_from_mut(value: &mut T) -> &mut (dyn Db + 'a) {
        value
    }
}
