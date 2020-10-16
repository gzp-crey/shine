use crate::{
    core::{hlist::HFind, ids::SmallStringId},
    ecs::{
        resources::{
            FetchResource, IntoResourceClaim, ResourceClaims, ResourceQuery, Resources, TagClaim, TagMutClaim,
        },
        ECSError,
    },
    hlist_type,
};
use std::{any, borrow::Cow, convert::TryFrom};

pub type SystemName = SmallStringId<16>;

/// Systems scheduled for execution
pub trait System: Send + Sync {
    fn type_name(&self) -> Cow<'static, str>;
    fn name(&self) -> &Option<SystemName>;
    //fn dependencies(&self) -> &Vec<SystemName>;
    fn resource_claims(&self) -> &ResourceClaims;
    fn run(&mut self, resources: &Resources) -> Result<(), ECSError>;
}

/// Trait to convert anything into a System.
/// The R genereic parameter is a tuple of all the resource queries
pub trait IntoSystem<R> {
    fn into_system(self) -> Result<Box<dyn System>, ECSError>;
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
    pub fn with_claim<T, Index>(mut self, claim: T) -> Self
    where
        C: HFind<T, Index>,
    {
        *self.claims.get_mut() = claim;
        self
    }
}

/// Helper trait to set the tags for shared tagged resource claims
pub trait WithTag<C, Index> {
    fn with_tag<Claim: 'static>(self, claim: &[&str]) -> Self
    where
        Self: Sized,
        C: HFind<TagClaim<Claim>, Index>;
}

impl<Func, C, R, Index> WithTag<C, Index> for SystemBuilder<Func, C, R> {
    fn with_tag<Claim: 'static>(mut self, claim: &[&str]) -> Self
    where
        Self: Sized,
        C: HFind<TagClaim<Claim>, Index>,
    {
        *self.claims.get_mut() = TagClaim::<Claim>::try_from(claim).unwrap();
        self
    }
}

/// Trait to set the tags for unique tagged resource claims
pub trait WithTagMut<C, Index> {
    fn with_tag_mut<Claim>(self, claim: &[&str]) -> Self
    where
        Self: Sized,
        Claim: 'static,
        C: HFind<TagMutClaim<Claim>, Index>;
}

impl<Func, C, R, Index> WithTagMut<C, Index> for SystemBuilder<Func, C, R> {
    fn with_tag_mut<Claim: 'static>(mut self, claim: &[&str]) -> Self
    where
        Self: Sized,
        Claim: 'static,
        C: HFind<TagMutClaim<Claim>, Index>,
    {
        *self.claims.get_mut() = TagMutClaim::<Claim>::try_from(claim).unwrap();
        self
    }
}

pub struct SystemFn<Func, Claims>
where
    Func: FnMut(&Resources, &Claims) -> Result<(), ECSError>,
{
    type_name: Cow<'static, str>,
    name: Option<SystemName>,
    claims: Claims,
    resource_claims: ResourceClaims,
    func: Func,
}

impl<Func, Claims> System for SystemFn<Func, Claims>
where
    Func: FnMut(&Resources, &Claims) -> Result<(), ECSError> + Send + Sync,
    Claims: 'static + Send + Sync,
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

    fn run(&mut self, resources: &Resources) -> Result<(), ECSError> {
        log::trace!("Running system [{:?}] - {:?}", self.name, self.type_name());
        (self.func)(resources, &self.claims)
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
            fn into_system(self) -> Result<Box<dyn System>, ECSError> {
                let SystemBuilder{ mut func, name, claims, .. } = self;
                let type_name = any::type_name::<Func>().into();
                let mut resource_claims = ResourceClaims::default();

                $(resource_claims.add_claim(<$resource as ResourceQuery>::default_claims());)*
                $(resource_claims.add_claim({
                        let $resource : &<$resource as ResourceQuery>::Claim = claims.get();
                        $resource.into_claim()?
                    });)*

                Ok(Box::new(SystemFn {
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
                            )?;
                        )*
                        fn_call!(func, ($($resource),*));
                        Ok(())
                    }
                }))
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
