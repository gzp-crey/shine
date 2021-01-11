# Entity component system

## Resource claims

| Reference         | Store | Entity | Desc
|-------------------|-------|--------|-------
| Store\<T>         | RL    | RL     | Claim exclusive access for the whole resource store of a type.
| Res\<T>           | RL    | RL     | Claim shared access for a single resource.
| ResMut\<T>        | RL    | WL     | Claim exclusive access for a single resource.
| MultiRes\<T>      | RL    | RL     | Claim shared access for multiple resources of the same type.
| MultiResMut\<T>   | RL    | WL     | Claim exclusive access for multiple resources of the same type.
| Local\<T>         | RL    | WL     | Claim exclusive access for a resource bound to a system (SystemId) (TBD)

## Schedule

- Prepare
  - For each task
    - Collect claims
    - Update order
- Run
  - While task
    - Find task that can be run (in parallel)
      - Start as much task as possible (based on task precondition and thread pool status)
        - Fetch (lock) resources, update resource location hints
        - Run system and collect new subtasks
        - Unlock resources
    - Await completed tasks
      - Enqueue the new subtasks (if any)
      - Update task preconditions

### Optimization

- To speed up resource claim we can cache the result of previous lookup as a ResourceHandle
  ex in `pub struct MultiResClaim<T: Resource>(Vec<ResourceId, Option<ResourceHandle<T>>>, PhantomData<fn(T)>);`
  If hint is present, we can use it (no hash lookup). If not, fetch it in the usual way, but update the hint.
  To be considered: ResourceHandle will keep resources alive, thus all resource referenced by a task will not be garbage collected
  by default.
- Resource exclusivity for task ordering can use a hash of the ResourceId, no need for the whole id (think of the Binary Keys).
  In case of conflict the worst thing that may happen is a reduced parallelism as the hash conflict may generate false
  conflicts. (ResourceIdHash can be stored/created in the system in the ResourceClaim, that usually has a longer lifetime)
