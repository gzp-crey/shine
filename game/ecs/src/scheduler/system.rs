use crate::{
    core::ids::SmallStringId,
    resources::{FetchResource, ResourceClaims, ResourceQuery, Resources},
};
use std::{any, borrow::Cow};

pub type SystemName = SmallStringId<16>;

/// Systems scheduled for execution
pub trait System: Send + Sync {
    fn type_name(&self) -> Cow<'static, str>;
    fn name(&self) -> &Option<SystemName>;
    fn resource_claims(&self) -> &ResourceClaims;
    fn run(&mut self, resources: &Resources);
}

/// Trait to convert anything into a System.
/// The R genereic parameter is a tuple of all the resource queries
pub trait IntoSystem<State, R> {
    fn into_system(self) -> Box<dyn System>;
}

/// Convert a sytem candidate into a sytem builder. It enables one to add extra
/// scheduling parameters.
pub trait IntoSystemBuilder<State, R> {
    type Builder: IntoSystem<State, R>;

    #[must_use]
    fn system(self) -> Self::Builder;
}

pub struct SystemBuilder<Func, R> {
    func: Func,
    name: Option<SystemName>,
    resource_claims: ResourceClaims,
    dependencies: Vec<SystemName>,
    _phantom: std::marker::PhantomData<R>,
}

impl<Func, R> SystemBuilder<Func, R> {
    #[must_use]
    pub fn with_name(mut self, name: Option<SystemName>) -> Self {
        self.name = name;
        self
    }

    #[must_use]
    pub fn with_resources<T: ResourceQuery>(mut self, claims: <T as ResourceQuery>::Claim) -> Self {
        <T as ResourceQuery>::add_extra_claim(claims, &mut self.resource_claims);
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
    Func: FnMut(&Resources, &mut State, &ResourceClaims),
    State: Sync + Send,
{
    type_name: Cow<'static, str>,
    name: Option<SystemName>,
    resource_claims: ResourceClaims,
    state: State,
    func: Func,
}

impl<State, Func> System for SystemFn<State, Func>
where
    Func: FnMut(&Resources, &mut State, &ResourceClaims) + Send + Sync,
    State: Sync + Send,
{
    fn type_name(&self) -> Cow<'static, str> {
        self.type_name.clone()
    }

    fn name(&self) -> &Option<SystemName> {
        &self.name
    }

    fn resource_claims(&self) -> &ResourceClaims {
        &self.resource_claims
    }

    fn run(&mut self, resources: &Resources) {
        log::trace!("Running system [{:?}] - {:?}", self.name, self.type_name());
        (self.func)(resources, &mut self.state, &self.resource_claims);
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
        impl<Func, $($resource,)*> IntoSystemBuilder<($($commands,)*), ($($resource,)*)> for Func
        where
            Func:
                FnMut($($commands,)* $($resource,)*) +
                FnMut(
                    $($commands,)*
                    $(<<$resource as ResourceQuery>::Fetch as FetchResource<'_>>::Item,)*) +
                Send + Sync + 'static,
            $($resource: ResourceQuery,)*
        {
            type Builder = SystemBuilder<Func, ($($resource,)*)>;

            fn system(self) -> Self::Builder {
                SystemBuilder {
                    name: None,
                    func: self,
                    resource_claims: Default::default(),
                    dependencies: Default::default(),
                    _phantom: std::marker::PhantomData,
                }
            }
        }

        impl<Func, $($resource,)*> IntoSystem<($($commands,)*), ($($resource,)*)> for SystemBuilder<Func, ($($resource,)*)>
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
                let SystemBuilder{ mut func, name, mut resource_claims, .. } = self;
                let type_name = any::type_name::<Func>().into();

                $(<$resource as ResourceQuery>::add_default_claim(&mut resource_claims);)*
                log::debug!("Resource claims for [{}]: {:#?}", type_name, resource_claims);

                Box::new(SystemFn {
                    name,
                    type_name,
                    resource_claims,
                    state: (),
                    func: move |resources, state, resource_claims| {
                        $(let mut $resource = <<$resource as ResourceQuery>::Fetch as FetchResource>::fetch(resources, resource_claims);)*
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
