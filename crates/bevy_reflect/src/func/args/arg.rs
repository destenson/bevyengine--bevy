use crate::func::args::{ArgError, FromArg, Ownership};
use crate::{PartialReflect, Reflect, TypePath};
use std::ops::Deref;

/// Represents an argument that can be passed to a [`DynamicFunction`], [`DynamicClosure`],
/// or [`DynamicClosureMut`].
///
/// [`DynamicFunction`]: crate::func::DynamicFunction
/// [`DynamicClosure`]: crate::func::DynamicClosure
/// [`DynamicClosureMut`]: crate::func::DynamicClosureMut
#[derive(Debug)]
pub struct Arg<'a> {
    index: usize,
    value: ArgValue<'a>,
}

impl<'a> Arg<'a> {
    /// Create a new [`Arg`] with the given index and value.
    pub fn new(index: usize, value: ArgValue<'a>) -> Self {
        Self { index, value }
    }

    /// The index of the argument.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Set the index of the argument.
    pub(crate) fn set_index(&mut self, index: usize) {
        self.index = index;
    }

    /// The value of the argument.
    pub fn value(&self) -> &ArgValue<'a> {
        &self.value
    }

    /// Take the value of the argument.
    pub fn take_value(self) -> ArgValue<'a> {
        self.value
    }

    /// Take the value of the argument and attempt to convert it to a concrete value, `T`.
    ///
    /// This is a convenience method for calling [`FromArg::from_arg`] on the argument.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::ArgList;
    /// let a = 1u32;
    /// let b = 2u32;
    /// let mut c = 3u32;
    /// let mut args = ArgList::new().push_owned(a).push_ref(&b).push_mut(&mut c);
    ///
    /// let a = args.take::<u32>().unwrap();
    /// assert_eq!(a, 1);
    ///
    /// let b = args.take::<&u32>().unwrap();
    /// assert_eq!(*b, 2);
    ///
    /// let c = args.take::<&mut u32>().unwrap();
    /// assert_eq!(*c, 3);
    /// ```
    pub fn take<T: FromArg>(self) -> Result<T::This<'a>, ArgError> {
        T::from_arg(self)
    }

    /// Returns `Ok(T)` if the argument is [`ArgValue::Owned`].
    ///
    /// If the argument is not owned, returns an error.
    ///
    /// It's generally preferred to use [`Self::take`] instead of this method.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::ArgList;
    /// let value = 123u32;
    /// let mut args = ArgList::new().push_owned(value);
    /// let value = args.take_owned::<u32>().unwrap();
    /// assert_eq!(value, 123);
    /// ```
    pub fn take_owned<T: Reflect + TypePath>(self) -> Result<T, ArgError> {
        match self.value {
            ArgValue::Owned(arg) => arg.try_take().map_err(|arg| ArgError::UnexpectedType {
                index: self.index,
                expected: std::borrow::Cow::Borrowed(T::type_path()),
                received: std::borrow::Cow::Owned(arg.reflect_type_path().to_string()),
            }),
            ArgValue::Ref(_) => Err(ArgError::InvalidOwnership {
                index: self.index,
                expected: Ownership::Owned,
                received: Ownership::Ref,
            }),
            ArgValue::Mut(_) => Err(ArgError::InvalidOwnership {
                index: self.index,
                expected: Ownership::Owned,
                received: Ownership::Mut,
            }),
        }
    }

    /// Returns `Ok(&T)` if the argument is [`ArgValue::Ref`].
    ///
    /// If the argument is not a reference, returns an error.
    ///
    /// It's generally preferred to use [`Self::take`] instead of this method.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::ArgList;
    /// let value = 123u32;
    /// let mut args = ArgList::new().push_ref(&value);
    /// let value = args.take_ref::<u32>().unwrap();
    /// assert_eq!(*value, 123);
    /// ```
    pub fn take_ref<T: Reflect + TypePath>(self) -> Result<&'a T, ArgError> {
        match self.value {
            ArgValue::Owned(_) => Err(ArgError::InvalidOwnership {
                index: self.index,
                expected: Ownership::Ref,
                received: Ownership::Owned,
            }),
            ArgValue::Ref(arg) => {
                Ok(arg
                    .try_downcast_ref()
                    .ok_or_else(|| ArgError::UnexpectedType {
                        index: self.index,
                        expected: std::borrow::Cow::Borrowed(T::type_path()),
                        received: std::borrow::Cow::Owned(arg.reflect_type_path().to_string()),
                    })?)
            }
            ArgValue::Mut(_) => Err(ArgError::InvalidOwnership {
                index: self.index,
                expected: Ownership::Ref,
                received: Ownership::Mut,
            }),
        }
    }

    /// Returns `Ok(&mut T)` if the argument is [`ArgValue::Mut`].
    ///
    /// If the argument is not a mutable reference, returns an error.
    ///
    /// It's generally preferred to use [`Self::take`] instead of this method.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::ArgList;
    /// let mut value = 123u32;
    /// let mut args = ArgList::new().push_mut(&mut value);
    /// let value = args.take_mut::<u32>().unwrap();
    /// assert_eq!(*value, 123);
    /// ```
    pub fn take_mut<T: Reflect + TypePath>(self) -> Result<&'a mut T, ArgError> {
        match self.value {
            ArgValue::Owned(_) => Err(ArgError::InvalidOwnership {
                index: self.index,
                expected: Ownership::Mut,
                received: Ownership::Owned,
            }),
            ArgValue::Ref(_) => Err(ArgError::InvalidOwnership {
                index: self.index,
                expected: Ownership::Mut,
                received: Ownership::Ref,
            }),
            ArgValue::Mut(arg) => {
                let received = std::borrow::Cow::Owned(arg.reflect_type_path().to_string());
                Ok(arg
                    .try_downcast_mut()
                    .ok_or_else(|| ArgError::UnexpectedType {
                        index: self.index,
                        expected: std::borrow::Cow::Borrowed(T::type_path()),
                        received,
                    })?)
            }
        }
    }
}

/// Represents an argument that can be passed to a [`DynamicFunction`].
///
/// [`DynamicFunction`]: crate::func::DynamicFunction
#[derive(Debug)]
pub enum ArgValue<'a> {
    Owned(Box<dyn PartialReflect>),
    Ref(&'a dyn PartialReflect),
    Mut(&'a mut dyn PartialReflect),
}

impl<'a> Deref for ArgValue<'a> {
    type Target = dyn PartialReflect;

    fn deref(&self) -> &Self::Target {
        match self {
            ArgValue::Owned(arg) => arg.as_ref(),
            ArgValue::Ref(arg) => *arg,
            ArgValue::Mut(arg) => *arg,
        }
    }
}