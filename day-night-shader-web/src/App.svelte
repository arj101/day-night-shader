<script lang="ts">
    import GlslCanvas from "glslCanvas";
    import { onMount } from "svelte";

    let canvasElt;
    let lat;
    let lon;

    $: latRadian = (parseFloat(lat) * Math.PI) / 180;
    $: lonRadian = (parseFloat(lon) * Math.PI) / 180;

    let sandbox;

    onMount(async () => {
        sandbox = new GlslCanvas(canvasElt);

        const fragShader = await fetch("./day-night-shader.frag");
        const fragShaderSrc = await fragShader.text();

        sandbox.load(fragShaderSrc);

        sandbox.setUniform("u_lat", 0.0);
        sandbox.setUniform("u_lon", 0.0);
        sandbox.setUniform("u_map_day", "./earth_day.png");
        sandbox.setUniform("u_map_night", "./earth_night.png");
    });
</script>

<main class="text-center p-4 mx-0">
    <canvas width="1100" height="550" bind:this={canvasElt} />
    <input
        type="range"
        min="-180"
        max="180"
        bind:value={lon}
        on:input={() => sandbox.setUniform("u_lon", lonRadian)}
    />
    <input
        type="range"
        min="-90"
        max="90"
        bind:value={lat}
        on:input={() => sandbox.setUniform("u_lat", latRadian)}
    />
</main>

<style lang="postcss">
</style>
