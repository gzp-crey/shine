{
    "primitive_topology": "TriangleList",
    "vertex_stage": {
        "shader": "./hello.vs",
        "attributes": [
            [0, "Position","Float3"],
            [1, {"TexCoord":0},"Float2"]
        ],
        "uniforms":[
            [2, [[0, {"Texture":"Diffuse"}],
                 [1, {"Sampler":"Diffuse"}]]]
        ]
    },
    "fragment_stage": {
        "shader": "./hello.fs",
        "uniforms": [
            [2, [[0, {"Texture":"Diffuse"}]]],
            [3, [[2, {"Sampler":"Diffuse"}]]]
        ]
    },
    "color_stage": "Replace"
}