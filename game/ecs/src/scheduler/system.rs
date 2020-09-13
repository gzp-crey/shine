use crate::{
    core::ids::SmallStringId,
    resources::{FetchResource, MultiResourceClaims, ResourceAccess, ResourceName, ResourceQuery, Resources},
};
use std::{any, borrow::Cow};

pub type SystemName = SmallStringId<16>;

/// Systems scheduled for execution
pub trait System: Send + Sync {
    fn type_name(&self) -> Cow<'static, str>;
    fn name(&self) -> &Option<SystemName>;
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
    fn system(self) -> Self::Configuration;
}

pub struct SystemConfiguration<Func, R> {
    func: Func,
    name: Option<SystemName>,
    multi_resource_claims: MultiResourceClaims,
    dependencies: Vec<SystemName>,
    _phantom: std::marker::PhantomData<R>,
}

impl<Func, R> SystemConfiguration<Func, R> {
    #[must_use]
    pub fn with_name(mut self, name: Option<SystemName>) -> Self {
        self.name = name;
        self
    }

    #[must_use]
    pub fn with_resources<T: ResourceQuery>(mut self, names: &[Option<ResourceName>]) -> Self {
        <T as ResourceQuery>::add_claim(&mut self.multi_resource_claims, names);
        self
    }

    #[must_use]
    pub fn with_dependencies(mut self, names: &[SystemName]) -> Self {
        self.dependencies.extend(names.iter().cloned());
        self
    }
}

pub struct SystemFn<State, Func>
where
    Func: FnMut(&Resources, &mut State, &ResourceAccess),
    State: Sync + Send,
{
    type_name: Cow<'static, str>,
    name: Option<SystemName>,
    resource_access: ResourceAccess,
    state: State,
    func: Func,
}

impl<State, Func> System for SystemFn<State, Func>
where
    Func: FnMut(&Resources, &mut State, &ResourceAccess) + Send + Sync,
    State: Sync + Send,
{
    fn type_name(&self) -> Cow<'static, str> {
        self.type_name.clone()
    }

    fn name(&self) -> &Option<SystemName> {
        &self.name
    }

    fn resource_access(&self) -> &ResourceAccess {
        &self.resource_access
    }

    fn run(&mut self, resources: &Resources) {
        (self.func)(resources, &mut self.state, &self.resource_access);
    }
}

macro_rules! fn_call {
    ($func:ident, ($($commands: ident, $commands_var: ident)*), ($($resource: ident),*)) => {
        $func($($commands,)* $($resource,)*)
    };
    ($self:ident, (), ($($resource: ident),*), ($($a: ident),*)) => {
        $func($($resource,)*)
    };
}

macro_rules! impl_into_system {
    (($($commands: ident)*), ($($resource: ident),*)) => {
        impl<Func, $($resource,)*> IntoSystemConfiguration<($($commands,)*), ($($resource,)*)> for Func
        where
            Func:
                FnMut($($commands,)* $($resource,)*) +
                FnMut(
                    $($commands,)*
                    $(<<$resource as ResourceQuery>::Fetch as FetchResource<'_>>::Item,)*) +
                Send + Sync + 'static,
            $($resource: ResourceQuery,)*
        {
            type Configuration = SystemConfiguration<Func, ($($resource,)*)>;

            fn system(self) -> Self::Configuration {
                SystemConfiguration {
                    name: None,
                    func: self,
                    multi_resource_claims: Default::default(),
                    dependencies: Default::default(),
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
                    $(<<$resource as ResourceQuery>::Fetch as FetchResource<'_>>::Item,)*) +
                Send + Sync + 'static,
                $($resource: ResourceQuery,)*
        {
            #[allow(unused_mut)]
            #[allow(unused_variables)]
            #[allow(non_snake_case)]
            fn into_system(self) -> Box<dyn System> {
                let SystemConfiguration{ mut func, name, multi_resource_claims, .. } = self;
                let type_name = any::type_name::<Func>().into();

                let mut resource_access = ResourceAccess::default();
                $(<<$resource as ResourceQuery>::Fetch as FetchResource<'_>>::access(&multi_resource_claims, &mut resource_access);)*

                log::debug!("Immutable resource claims for [{}]: {:?}", type_name, resource_access.get_immutable_multi_claims());
                log::debug!("Mutable resource claims for [{}]: {:?}", type_name, resource_access.get_mutable_multi_claims());

                Box::new(SystemFn {
                    name,
                    type_name,
                    resource_access,
                    state: (),
                    func: move |resources, state, resource_access| {
                        $(let mut $resource = <<$resource as ResourceQuery>::Fetch as FetchResource>::fetch(resources, resource_access);)*
                        fn_call!(func, ($($commands, state)*), ($($resource),*));
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
impl_into_systems!(Ra);
impl_into_systems!(Ra, Rb);
impl_into_systems!(Ra, Rb, Rc);
impl_into_systems!(Ra, Rb, Rc, Rd);
impl_into_systems!(Ra, Rb, Rc, Rd, Re);
impl_into_systems!(Ra, Rb, Rc, Rd, Re, Rf);
impl_into_systems!(Ra, Rb, Rc, Rd, Re, Rf, Rg);
impl_into_systems!(Ra, Rb, Rc, Rd, Re, Rf, Rg, Rh);
impl_into_systems!(Ra, Rb, Rc, Rd, Re, Rf, Rg, Rh, Ri);
impl_into_systems!(Ra, Rb, Rc, Rd, Re, Rf, Rg, Rh, Ri, Rj);
