{
    "primitive_topology": "TriangleList",
    "vertex_stage": {
        "shader": "pipeline/hello.vs",
        "attributes": {
            "2": {
                "Custom": [
                    "c1",
                    {
                        "Float3a": 16
                    }
                ]
            },
            "1": "Norm3",
            "0": "Pos3"
        },
        "global_uniforms": {
            "0": "ModelView"
        },
        "local_uniforms": {
            "0": "ModelView"
        }
    },
    "fragment_stage": {
        "shader": "pipeline/hello.fs",
        "global_uniforms": {
            "0": "ModelView"
        },
        "local_uniforms": {}
    },
    "color_stage": "Replace"
}