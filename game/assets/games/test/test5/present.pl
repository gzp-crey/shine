{
    "primitive_topology": "TriangleList",
    "vertex_stage": {
        "shader": "./present.vs",
        "attributes": [],
        "auto_uniforms": [],
        "global_uniforms": [],
        "local_uniforms": []
    },
    "fragment_stage": {
        "shader": "./present.fs",
        "auto_uniforms": [
            [1,{"Texture": {"Frame": "frame_color"}}],
            [2,{"Sampler": {"Frame": "frame_color"}}]
        ],
        "global_uniforms": [],
        "local_uniforms": []
    },
    "color_stage": "Replace"
}