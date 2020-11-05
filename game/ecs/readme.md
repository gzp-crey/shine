# Entity component system

Credit goes to the implementors and mainteners of the hecs, legion and Yaks crates.

## Resource types

| Reference         | Store | Entity | Desc
|-------------------|-------|--------|-------
| Store\<T>         | RL    | RL     | Claim shared access for the whole store, no update on store or on any resource
| StoreMut\<T>      | WL    | WL     | Claim exclusive access for the whole store, can create/destroy/update resources
| Res\<T>           | RL    | RL     | Claim shared access for a single resource
| ResMut\<T>        | RL    | WL     | Claim exclusive access for a single resource
| MultiRes\<T>      | RL    | RL     | Claim shared access for multiple resources of the same type
| MultiResMut\<T>   | RL    | WL     | Claim exclusive access for multiple resources of the same type
| Local\<T>         | RL    | WL     | Claim exclusive access for a resource bound to a system (SystemId)

## Schedule

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
