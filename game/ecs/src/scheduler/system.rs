use crate::resources::{FetchResource, MultiResourceClaims, Resource, ResourceAccess, ResourceQuery, Resources};
use std::{
    any::{self, TypeId},
    borrow::Cow,
    collections::HashMap,
};

/// Systems scheduled for execution
pub trait System: Send + Sync {
    fn type_name(&self) -> Cow<'static, str>;
    fn name(&self) -> Option<&str>;
    fn resource_access(&self) -> &ResourceAccess;
    fn run(&mut self, resources: &Resources);
}

/// Trait to convert anything into a System.
/// The R genereic parameter is a tuple of all the resource queries
pub trait IntoSystem<State, R> {
    fn into_system(self) -> Box<dyn System>;
}

/// Convert a sytem candidate into a sytem configuration. It enables one to add extra
/// scheduling parameters.
pub trait IntoSystemConfiguration<State, R> {
    type Configuration: IntoSystem<State, R>;

    #[must_use]
    fn configure(self) -> Self::Configuration;
}

/// Blanket implementation to create a system without configuration
impl<Func, State, R> IntoSystem<State, R> for Func
where
    Func: IntoSystemConfiguration<State, R>,
{
    #[must_use]
    fn into_system(self) -> Box<dyn System> {
        self.configure().into_system()
    }
}

pub struct SystemConfiguration<Func, R> {
    func: Func,
    name: Option<String>,
    multi_resource_claims: MultiResourceClaims,
    _phantom: std::marker::PhantomData<R>,
}

impl<Func, R> SystemConfiguration<Func, R> {
    #[must_use]
    pub fn with_name<T: Resource>(mut self, name: Option<String>) -> Self {
        self.name = name;
        self
    }

    #[must_use]
    pub fn with_resources<T: Resource>(mut self, names: &[Option<String>]) -> Self {
        let ty = TypeId::of::<T>();
        self.multi_resource_claims
            .immutable
            .entry(ty)
            .or_default()
            .extend(names.iter().cloned());
        self
    }

    #[must_use]
    pub fn with_mut_resources<T: Resource>(mut self, names: &[Option<String>]) -> Self {
        let ty = TypeId::of::<T>();
        self.multi_resource_claims
            .mutable
            .entry(ty)
            .or_default()
            .extend(names.iter().cloned());
        self
    }
}

pub struct SystemFn<State, Func>
where
    Func: FnMut(&Resources, &ResourceAccess),
    State: Sync + Send,
{
    type_name: Cow<'static, str>,
    name: Option<String>,
    resource_access: ResourceAccess,
    state: State,
    func: Func,
}

impl<State, Func> System for SystemFn<State, Func>
where
    Func: FnMut(&Resources, &ResourceAccess) + Send + Sync,
    State: Sync + Send,
{
    fn type_name(&self) -> Cow<'static, str> {
        self.type_name().clone()
    }

    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn resource_access(&self) -> &ResourceAccess {
        &self.resource_access
    }

    fn run(&mut self, resources: &Resources) {
        (self.func)(resources, &self.resource_access);
    }
}

macro_rules! impl_into_system {
    (($($commands: ident)*), ($($resource: ident),*)) => {
        impl<Func, $($resource,)*> IntoSystemConfiguration<($($commands,)*), ($($resource,)*)> for Func
        where
            Func:
                FnMut($($commands,)* $($resource,)*) +
                FnMut(
                    $($commands,)*
                    $(<<$resource as ResourceQuery>::Fetch as FetchResource>::Item,)*) +
                Send + Sync + 'static,
            $($resource: ResourceQuery,)*
        {
            type Configuration = SystemConfiguration<Func, ($($resource,)*)>;

            fn configure(self) -> Self::Configuration {
                SystemConfiguration {
                    name: None,
                    func: self,
                    multi_resource_claims: MultiResourceClaims{
                        immutable: HashMap::default(),
                        mutable: HashMap::default(),
                    },
                    _phantom: std::marker::PhantomData,
                }
            }
        }

        impl<Func, $($resource,)*> IntoSystem<($($commands,)*), ($($resource,)*)> for SystemConfiguration<Func, ($($resource,)*)>
        where
        Func:
            FnMut($($commands,)* $($resource,)*) +
            FnMut(
                $($commands,)*
                $(<<$resource as ResourceQuery>::Fetch as FetchResource>::Item,)*) +
            Send + Sync + 'static,
            $($resource: ResourceQuery,)*
        {
            fn into_system(self) -> Box<dyn System> {
                let mut resource_access = ResourceAccess::new();
                $(<<$resource as ResourceQuery>::Fetch as FetchResource>::access(&self.multi_resource_claims, &mut resource_access);)*

                Box::new(SystemFn {
                    name: self.name,
                    type_name : any::type_name::<Func>().into(),
                    resource_access,
                    state: (),
                    func: move |_, _| {
                        //let res = ($(<<$resource as ResourceQuery>::Fetch as FetchResource>::fetch(resources, claims),)*);
                        unimplemented!()
                    }
                })
            }
        }
    }
}

macro_rules! impl_into_systems {
    ($($resource: ident),*) => {
        impl_into_system!((), ($($resource),*));
        //impl_into_system!((Commands), ($($resource),*));
    };
}

impl_into_systems!();
//impl_into_systems!(Ra);
//impl_into_systems!(Ra, Rb);
//impl_into_systems!(Ra, Rb, Rc);
impl_into_systems!(Ra, Rb, Rc, Rd);
//impl_into_systems!(Ra, Rb, Rc, Rd, Re);
//impl_into_systems!(Ra, Rb, Rc, Rd, Re, Rf);
//impl_into_systems!(Ra, Rb, Rc, Rd, Re, Rf, Rg);
//impl_into_systems!(Ra, Rb, Rc, Rd, Re, Rf, Rg, Rh);
//impl_into_systems!(Ra, Rb, Rc, Rd, Re, Rf, Rg, Rh, Ri);
//impl_into_systems!(Ra, Rb, Rc, Rd, Re, Rf, Rg, Rh, Ri, Rj);

use std::sync::{Arc, Mutex};
pub struct Schedule2 {
    pub(crate) systems: Vec<Arc<Mutex<Box<dyn System>>>>,
}

impl Schedule2 {
    pub fn new() -> Schedule2 {
        Schedule2 {
            systems: Default::default(),
        }
    }

    pub fn schedule<State, R, Func: IntoSystem<State, R>>(&mut self, func: Func) {
        let system = func.into_system();
        self.systems.push(Arc::new(Mutex::new(system)));
    }
}

use crate::resources::{MultiRes, MultiResMut, Res, ResMut};
fn sys0() {
    unimplemented!()
}

fn sys4(r1: Res<usize>, r2: ResMut<String>, r3: MultiRes<u8>, r4: MultiResMut<u16>) {
    unimplemented!()
}

fn foo() {
    let mut sh = Schedule2::new();

    sh.schedule(
        sys0.configure()
            .with_resources::<MultiRes<u8>>(&[None])
            .with_resources::<MultiResMut<u16>>(&[None]),
    );

    sh.schedule(
        sys4.configure()
            .with_resources::<MultiRes<u8>>(&[None])
            .with_resources::<MultiResMut<u16>>(&[None]),
    );

    sh.schedule(sys4);
}
