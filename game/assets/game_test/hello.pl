{
    "primitive_topology": "TriangleList",
    "vertex_stage": {
        "shader": "./hello.vs",
        "attributes": [
            [0, "Position","Float3"],
            [1, {"TexCoord":0},"Float2"]
        ]
    },
    "fragment_stage": {
        "shader": "./hello.fs"
    },
    "color_stage": "Replace"
}