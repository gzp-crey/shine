{
    "primitive_topology": "TriangleList",
    "vertex_stage": {
        "shader": "./present.vs",
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
        "shader": "./present.fs",
		"uniforms": [
            [
                [2, {"Texture": {"Frame":"frame_color"}}],
                [3, {"Sampler": {"Frame":"frame_color"}}]
            ],
            []
        ]
    },
    "color_stage": "Replace"
}