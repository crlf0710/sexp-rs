#![feature(proc_macro)]
#![feature(get_type_id)]

extern crate gc;

#[macro_use]
extern crate gc_derive;

use std::mem;
use std::any::{Any, TypeId};
use gc::{Gc, GcCell, Trace};

#[derive(Trace)]
pub enum Value {
    Nil,
    Atom(Box<Atomic>),
    Cons(ConsCell),
}

impl Clone for Value {
    fn clone(&self) -> Self {
        match *self {
            Value::Nil => Value::Nil,
            Value::Atom(ref x) => Value::Atom(x.as_ref().clone_and_box()),
            Value::Cons(ref x) => Value::Cons(x.clone()),
        }
    }
}

unsafe impl Trace for Box<Atomic> {
    unsafe fn trace(&self) {
        self.as_ref().trace()
    }

    unsafe fn root(&self) {
        self.as_ref().root()
    }

    unsafe fn unroot(&self) {
        self.as_ref().unroot()
    }

    fn finalize_glue(&self) {
        self.as_ref().finalize_glue()
    }
}

pub trait Atomic: Any + Trace + 'static {
    fn clone_and_box(&self) -> Box<Atomic>;
}

impl<T> Atomic for T
    where T: Any + Clone + Trace + 'static
{
    fn clone_and_box(&self) -> Box<Atomic> {
        Box::new(self.clone())
    }
}

#[derive(Trace)]
pub struct ConsCellData {
    car: GcCell<Value>,
    cdr: GcCell<Value>,
}

pub type ConsCell = Gc<ConsCellData>;

pub fn cons(car: Value, cdr: Value) -> Value {
    Value::Cons(Gc::new(ConsCellData {
        car: GcCell::new(car),
        cdr: GcCell::new(cdr),
    }))
}

pub fn atom<T: Atomic>(v: T) -> Value {
    Value::Atom(Box::new(v))
}

pub fn nil() -> Value {
    Value::Nil
}

#[doc(hidden)]
#[derive(Clone, Copy)]
struct TraitObject {
    pub data: *mut (),
    pub vtable: *mut (),
}

#[doc(hidden)]
#[inline(always)]
fn to_trait_object<B: ?Sized>(base: &B) -> TraitObject {
    assert_eq!(mem::size_of::<&B>(), mem::size_of::<TraitObject>());
    unsafe { *mem::transmute::<&&B, *const TraitObject>(&base) }
}

unsafe fn downcast_ref_unchecked<'a, T: Atomic>(value: &'a Atomic) -> &'a T {
    ::std::mem::transmute(to_trait_object(value).data)
}

unsafe fn downcast_mut_unchecked<'a, T: Atomic>(value: &'a mut Atomic) -> &'a mut T {
    ::std::mem::transmute(to_trait_object(value).data)
}

impl Value {
    pub fn is_nil(&self) -> bool {
        if let &Value::Nil = self { true } else { false }
    }

    pub fn is_atom(&self) -> bool {
        if let &Value::Atom(_) = self {
            true
        } else {
            false
        }
    }

    pub fn atom_clone(&self) -> Option<Box<Atomic>> {
        if let &Value::Atom(ref a) = self {
            let c = (*a).clone_and_box();
            Some(c)
        } else {
            None
        }
    }

    pub fn atom_get_type_id(&self) -> Option<TypeId> {
        if let &Value::Atom(ref a) = self {
            Some((*a).as_ref().get_type_id())
        } else {
            None
        }
    }

    pub fn atom_downcast_ref<T: Atomic>(&self) -> Option<&T> {
        if let &Value::Atom(ref a) = self {
            let r = (*a).as_ref();
            if r.get_type_id() == TypeId::of::<T>() {
                Some(unsafe { downcast_ref_unchecked(r) })
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn atom_downcast_mut<T: Atomic>(&mut self) -> Option<&mut T> {
        if let &mut Value::Atom(ref mut a) = self {
            if (*a).as_ref().get_type_id() == TypeId::of::<T>() {
                let mut r = (*a).as_mut();
                let m = unsafe { downcast_mut_unchecked(r) };
                Some(m)
            } else {
                None
            }
        } else {
            None
        }
    }


    pub fn is_cons(&self) -> bool {
        if let &Value::Cons(_) = self {
            true
        } else {
            false
        }
    }

    pub fn car(&self) -> Option<Value> {
        if let &Value::Cons(ref c) = self {
            Some(c.car.borrow().clone())
        } else {
            None
        }
    }

    pub fn cdr(&self) -> Option<Value> {
        if let &Value::Cons(ref c) = self {
            Some(c.cdr.borrow().clone())
        } else {
            None
        }
    }

    pub fn set_car(&self, v: Value) -> Result<(), Value> {
        if let &Value::Cons(ref c) = self {
            *c.car.borrow_mut() = v;
            Ok(())
        } else {
            Err(v)
        }
    }

    pub fn set_cdr(&self, v: Value) -> Result<(), Value> {
        if let &Value::Cons(ref c) = self {
            *c.cdr.borrow_mut() = v;
            Ok(())
        } else {
            Err(v)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let lst = cons(atom(3usize), cons(atom(4usize), nil()));
        assert_eq!(lst.car().unwrap().atom_downcast_ref::<usize>().unwrap(),
                   &3usize);
        assert_eq!(lst.cdr().unwrap().car().unwrap().atom_downcast_ref::<usize>().unwrap(),
                   &4usize);
        assert!(lst.cdr().unwrap().cdr().unwrap().is_nil());
    }
}
