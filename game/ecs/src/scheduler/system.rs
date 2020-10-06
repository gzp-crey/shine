use crate::{
    core::{
        hlist::{Find, HCons, HList},
        ids::SmallStringId,
    },
    hlist, hlist_type,
    resources::{FetchResource, IntoResourceClaim, ResourceClaims, ResourceQuery, Resources},
};
use std::{any, borrow::Cow};

pub type SystemName = SmallStringId<16>;

/// Systems scheduled for execution
pub trait System: Send + Sync {
    fn type_name(&self) -> Cow<'static, str>;
    fn name(&self) -> &Option<SystemName>;
    //fn dependencies(&self) -> &Vec<SystemName>;
    fn resource_claims(&self) -> &ResourceClaims;
    fn run(&mut self, resources: &Resources);
}

/// Trait to convert anything into a System.
/// The R genereic parameter is a tuple of all the resource queries
pub trait IntoSystem<R> {
    fn into_system(self) -> Box<dyn System>;
}

/// Trait to convert anything into a (system) Builder. Before constructing the system one may add extra
/// scheduling parameters.
pub trait IntoSystemBuilder<R> {
    type Builder: IntoSystem<R>;

    #[must_use]
    fn system(self) -> Self::Builder;
}

pub struct SystemBuilder<Func, C, R> {
    func: Func,
    name: Option<SystemName>,
    dependencies: Vec<SystemName>,
    claims: C,
    _phantom: std::marker::PhantomData<R>,
}

impl<Func, C, R> SystemBuilder<Func, C, R> {
    #[must_use]
    pub fn with_name(mut self, name: Option<SystemName>) -> Self {
        self.name = name;
        self
    }

    #[must_use]
    pub fn with_dependencies(mut self, names: &[SystemName]) -> Self {
        self.dependencies.extend(names.iter().cloned());
        self
    }

    #[must_use]
    pub fn claim<F: FnOnce(&mut C)>(mut self, claim: F) -> Self {
        (claim)(&mut self.claims);
        self
    }
}

pub struct SystemFn<Func, Claims>
where
    Func: FnMut(&Resources, &Claims),
{
    type_name: Cow<'static, str>,
    name: Option<SystemName>,
    claims: Claims,
    resource_claims: ResourceClaims,
    func: Func,
}

impl<Func, Claims> System for SystemFn<Func, Claims>
where
    Func: FnMut(&Resources, &Claims) + Send + Sync,
    Claims: 'static + Send + Sync
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
        (self.func)(resources, &self.claims);
    }
}

macro_rules! fn_call {
    ($func:ident, ($($resource: ident),*)) => {
        $func($($resource,)*)
    };
}

macro_rules! impl_into_system {
    (($($resource: ident),*)) => {
        impl<Func, $($resource,)*> IntoSystemBuilder<($($resource,)*)> for Func
        where
            Func:
                FnMut($($resource,)*) +
                FnMut($(<<$resource as ResourceQuery>::Fetch as FetchResource<'_, <$resource as ResourceQuery>::Claim>>::Item,)*)
                + Send + Sync + 'static,
            $($resource: ResourceQuery,)*
        {
            type Builder = SystemBuilder<Func,
                             hlist_type![$(<$resource as ResourceQuery>::Claim,)*],
                             ($($resource,)*)>;

            fn system(self) -> Self::Builder {
                SystemBuilder {
                    name: None,
                    func: self,
                    dependencies: Default::default(),
                    _phantom: std::marker::PhantomData,
                    claims: Default::default(),
                }
            }
        }

        impl<Func, $($resource,)*> IntoSystem<($($resource,)*)>
            for SystemBuilder<
                    Func,
                    hlist_type![$(<$resource as ResourceQuery>::Claim,)*],
                    ($($resource,)*)>
        where
            Func:
                FnMut( $($resource,)*) +
                FnMut( $(<<$resource as ResourceQuery>::Fetch as FetchResource<'_, <$resource as ResourceQuery>::Claim>>::Item,)*) +
                Send + Sync + 'static,
            $($resource: ResourceQuery,)*
        {
            #[allow(unused_mut)]
            #[allow(unused_variables)]
            #[allow(non_snake_case)]
            fn into_system(self) -> Box<dyn System> {
                let SystemBuilder{ mut func, name, claims, .. } = self;
                let type_name = any::type_name::<Func>().into();
                let mut resource_claims = ResourceClaims::default();

                $(resource_claims.add_claim(<$resource as ResourceQuery>::default_claims());)*
                $(resource_claims.add_claim({
                        let $resource : &<$resource as ResourceQuery>::Claim = claims.get();
                        $resource.into_claim()
                    });)*

                Box::new(SystemFn {
                    name,
                    type_name,
                    resource_claims,
                    claims,
                    func: move |resources, claims| {
                        $(
                            let mut $resource = <<$resource as ResourceQuery>::Fetch as FetchResource<'_, <$resource as ResourceQuery>::Claim>>::fetch(
                                resources,
                                {
                                    let $resource : &<$resource as ResourceQuery>::Claim = claims.get();
                                    $resource
                                }
                            );
                        )*
                        fn_call!(func, ($($resource),*));
                    }
                })
            }
        }
    }
}

macro_rules! impl_into_systems {
    ($($resource: ident),*) => {
        impl_into_system!(($($resource),*));
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
