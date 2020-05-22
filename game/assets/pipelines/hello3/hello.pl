{
    "primitive_topology": "TriangleList",
    "vertex_stage": {
        "shader": "./hello.vs",
        "attributes": [[0,"Position","Float3"],[1,{"TexCoord":0},"Float2"]],
        "global_uniforms":[],
        "local_uniforms": []
    },
    "fragment_stage": {
        "shader": "./hello.fs",
		"global_uniforms":[[0,"Diffuse","Texture"],[1,"Diffuse","Sampler"]],
        "local_uniforms": []
    },
    "color_stage": "Replace"
}