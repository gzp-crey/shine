{
    "primitive_topology": "TriangleList",
    "vertex_stage": {
        "shader": "./present.vs",
        "attributes": [],
        "uniforms": [[],[]]
    },
    "fragment_stage": {
        "shader": "./present.fs",
        "uniforms": [
            [
                [1,{"Texture": {"Frame": "frame_color"}}],
                [2,{"Sampler": {"Frame": "frame_color"}}]
            ],
            []
        ]
    },
    "color_stage": "Replace"
}