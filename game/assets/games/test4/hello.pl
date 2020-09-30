{
    "primitive_topology": "TriangleList",
    "vertex_stage": {
        "shader": "./hello.vs",
        "attributes": [
            [0, "Position","Float3"],
            [1, {"TexCoord":0},"Float2"]
        ],
        "auto_uniforms": [],
        "global_uniforms": [
            [0, {"UniformBuffer":"ViewProj"}]
        ],
        "local_uniforms": []
    },
    "fragment_stage": {
        "shader": "./hello.fs",
        "auto_uniforms": [],
        "global_uniforms": [
            [1, {"Texture":"Diffuse"}],
            [2, {"Sampler":"Diffuse"}]
        ],
        "local_uniforms": []
    },
    "color_stage": "Replace"
}