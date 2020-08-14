{
    "primitive_topology": "TriangleList",
    "vertex_stage": {
        "shader": "./hello.vs",
        "attributes": [
            [0, "Position", "Float3"],
            [1, {"TexCoord":0}, "Float2"]
        ],
        "auto_uniforms": [],
        "global_uniforms": [],
        "local_uniforms": []
    },
    "fragment_stage": {
        "shader": "./hello.fs",
        "auto_uniforms": [],
        "global_uniforms": [
            [0, {"Texture":"Diffuse"}],
            [1, {"Sampler":"Diffuse"}]
        ],
        "local_uniforms": []
    },
    "color_stage": "Replace"
}