const rust = import('./pkg/shine_webgame');
const canvas = document.getElementById('gameCanvas');
const webgpu = canvas.getContext("gpupresent");


rust
    .then(
        async m => {
            if (!webgpu) {
                alert('Failed to initialize WebGPU');
                return;
            }

            const game = new m.WebGame;
            console.log(game);
            const gameView = await game.create_render('gameCanvas');
            console.log(gameView);
        })
    .catch(
        error => {
            alert('Failed to initialize game', error);
        });
