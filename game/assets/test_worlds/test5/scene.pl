{
    "primitive_topology": "TriangleList",
    "vertex_stage": {
        "shader": "./hello.vs",
        "attributes": [
            [0, "Position","Float3"],
            [1, {"TexCoord":0},"Float2"]
        ],
		"uniforms": [
            [
                [0, {"UniformBuffer":"ViewProj"}]
            ],
            []
        ]
    },
    "fragment_stage": {
        "shader": "./hello.fs",
		"uniforms": [
            [
                [2, {"Texture":{"Graph":"frame_color"}],
                [3, {"Sampler":{"Graph":"frame_depth"}]
            ],
            []
        ]
    },
    "color_stage": "Replace"
}