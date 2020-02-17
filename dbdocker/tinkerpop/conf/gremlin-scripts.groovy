// an init script that returns a Map allows explicit setting of global bindings.
def globals = [:]

// Generates the classic graph into an "empty" TinkerGraph via LifeCycleHook.
// Note that the name of the key in the "global" map is unimportant.
globals << [hook : [
  onStartUp: { ctx ->
    ctx.logger.info("Loading 'classic' graph data.")   
  }
] as LifeCycleHook]

// define the default TraversalSource to bind queries to - this one will be named "g".
// ReferenceElementStrategy converts all graph elements (vertices/edges/vertex properties)
// to "references" (i.e. just id and label without properties). this strategy was added
// in 3.4.0 to make all Gremlin Server results consistent across all protocols and
// serialization formats aligning it with TinkerPop recommended practices for writing
// Gremlin.
globals << [g : graph.traversal().withStrategies(ReferenceElementStrategy.instance())]
