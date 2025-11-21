# The Three Flatland Problem Master Thesis

This repo contains the code for my master's thesis, [The Three Flatland Problem](https://repository.tudelft.nl/record/uuid:f074c49c-2ee4-4234-8812-945945a8f760).
You can find a recording of me presenting the thesis to the VISGRAF Lab on [Youtube](https://www.youtube.com/live/6TUtI7KX0fA).
This repo contains the code for running the simulation, the factorization, and the benchmarks.

In essence, this research attempts to leverage the reduced dimension of 2D content to produce accurate focus cues when such content would be displayed in augmented reality.

This approach improves sampling time massively, and compute time when a kernel is used.


## Graphs:
Sep is our method; Stereo is state of the art.

![Ray casting time](/Results/Ray Casting.png)
![Total Time spent](/Results/Time Spent.png)




## Images Produced:
Target Image:
![Target 2D content](/pics/No kernel.png)
![What an observer would see with our kernel](/pics/Kernel View Far.png)
![Target](/pics/Kernel View Far.png)
![How the content gets split across both panels](/pics/Close Up to Kernel.png)


## Simulation
The simulation is done by complete raycasting on the GPU side. It abuses the fragment shader to cast the rays.


## Dependencies:
Ensure you have Rust installed.
[https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install)

## How to run?
**Simulation**
```cargo run -r```

**Benchmarks**
```cargo run -r -- --headless -t <TYPE>```
```-type-head <TYPE_HEAD>  [possible values: sep, sep-old, stereo, load]```

