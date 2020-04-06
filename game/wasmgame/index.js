const canvas = document.getElementById('gameCanvas');
const webgpu = canvas.getContext("gpupresent");

if (!webgpu) {
    alert('Failed to initialize WebGPU');
}
else {
    const rust = import('./pkg/shine_wasm');

    rust
        .then(
            async m => {
                game = new m.WebGame;
                console.log(game);
                gameView = await game.create_render('gameCanvas');
                console.log(gameView);
            })
        .catch(
            error => {
                console.log('Failed to initialize game', error);
            });
}