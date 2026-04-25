class Inspector
  constructor: (@document) ->
    @tree = []
  
  selectNode: (node) ->
    console.log "Selected node:", node
  
  render: ->
    """
    <div class="inspector">
      <h3>DOM Inspector</h3>
      <div class="tree"></div>
    </div>
    """

export { Inspector }
