# Entity component system

Credit goes to the implementors and mainteners of the hecs, legion and Yaks crates.

## Resources

### Resource types

|Reference          | Store | Entity | Desc
|-------------------|-------|--------|-------
| ?StoreCommand\<T> |       |        | Get SendCommandQueue for store, no lock as it is a sync object by nature
| Store\<T>         | RL    | RL     | Claim shared access for the whole store, no update on store or on any resource
| StoreMut\<T>      | WL    | WL     | Claim exclusive access for the whole store, can create/destroy/update resources
| Res\<T>           | RL    | RL     | Claim shared access for a single resource
| ResMut\<T>        | RL    | WL     | Claim exclusive access for a single resource
| Tag\<T>           | RL    | RL     | Claim shared access for multiple resource
| TagMut\<T>        | RL    | WL     | Claim exclusive access for multiple resource
| Local\<T>         | RL    | WL     | Claim exclusive access for a resource bound to a system

### Resource type construction and claim

As function argument types and related claim configuration
| Reference     | Construct                                                        | Extra
|---------------|------------------------------------------------------------------|-------
| Res[Mut]\<T>  | insert(T)                                                        |
|               | insert_with(Fn()->T, Fn(StoreCommand, WeakHandle, A))            |
| Tag[Mut]\<T>  | insert_tagged(tag, T)                                            | with_tags(["a","b"])
|               | insert_tagged_with(Fn(tag)->T, Fn(StoreCommand, WeakHandle, A))  |
| Local\<T>     | insert_local(Fn() -> T)                                          | "automatic" SystemId

Requesting from store:
fn Store::\<T>::get_handle(&self) -> Option<Handle>;
fn Store::\<T>::get_handle_with_tag(&self, tag)  -> Option<Handle>;

fn Store::\<T>::index(&self, &Handle)  -> Ref<T>
fn StoreMut::\<T>::index_mut(&mut self, &MutHandle) -> RefMut<T>

## Implementation details

//store the resource
struct UnsafeCell(RefCount, T)

struct Handle(*Store<T>, *T)
deref: unsafe cast from *T
clone: incerment atomic counter
drop: decrement atomic counter

?: How to create a "weak ref" from a Handle for the CommandQueue ?
  With store: Store<T>::request_async(&Handle, arg: A);

Store := TransientStore + SteadyStore + CommandQueue(mpsc)

### Store

- get_handle
  - Check SteadyStore
  - Check Transient store
  - Create in transient (if has builder)

### Asyn load

- request: StoreCommand + Weak handle to resource + arg(tag)
  - Perform load
  - Send response to store

### Schedule

- Prepare
  - For each store
    - Apply commands
  - For each task
    - Collect claims
    - Update order
- Run
  - For each task
    - Fetch resources
      - Check SteadStore
      - Check TransientStore
    - Run system
      - Collect commands (delete)
  - Release resources
- Post
  - For each store
    - Move from transient into steady
    - Release unused
