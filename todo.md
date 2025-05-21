# Todo:

Plane F is a texture (image).

So you place a pinhole camera at O, and ray trace the plane F, the ray
will intersect a pixel b at B and a pixel a at A.

Then you shoot a ray that goes from the center of b passing through the
center of a. The ray will hit the image at F, that is the sample you
want to compare against direct ray-tracing.

This difference will show what is the limit we can achieve with this
dual display idea. Basically, if the image of the red rays is horrible,
we are in trouble.

Create a small application to vary the parameters (distance to F,
resolution of A and B ... and so on)

## What I need:

Take in parameters (distance to F, resoluion of A and B, distance A from B)
Resolution determines how the planes are cut up into pixels.
Each pixel is a 

We build two images backwards:

The one that originates at 0 and the one that the pixel in B.


What is the difference between the images that are produced by sitting at 0 and
those the follow the ray more correctly



For every ray shot from O, get it's twin ray, that will be shot from the pixels
that pass.


Include ray count in parameters?

Produce two views / images: the difference in views

Inspect 3D world? Does that make any sense?

Do I march the ray? WE will see how 

The question is just where do we intersect and what pixel does that give us -->
color!


Perfect tool would allow us to place the camera in 3D space, hit a key and have
it done!

The question is how different does this work?


Shoot the ray that goes to center of each pixel in the projection.
Yeah this all makes sense!
The resolution of our target images matters here as well basically


## First:
Code needs to be refactored to be more flexible
In particular, reconsider how elements are placed

Extend raytracer to deal with semi opaquee object
Extend ray tracer to be concurrent?




## How to find the pixel and pixel center:
Transform the intersection into quad cords
Multiply by the size will give us the "pixel coordinates"
Rounding will give us the nearest pixel (probably minus 1?)
Center doable

Then need to translate this back to view space


How quad space looks:

0,0 ----- 1,0
 |         |
 |         |
 |         |
0,1-------1,1     


## AP:

Pixel disparity between rays as an image
Save state
Replay with inner script

Kick off papers

Write a proposal, "what is the problem, what is the motivation"
What is the background (about 1 page)

Make a plane of action, main thing: 

What should be ready after 3 months?
The simulation should be ready. Given a target, we should have the image
decomposition ready

What to expect to deliver at the end?
We want results tested on real hardware


Mare

## Writing:
Take notes for summary for related work. 

Week 3 objectives:

Proposal write up

Get credits DONE!

make targets for planning Mostly done



work on template 


## Pixel bug writing:
Seems like the "find center of pixel" doesn't actually work!
it is doing collisions with invisible geometry, and therefore needs to be
revised!

Proposal:
Solve the problem on paper first, with tests
Then, do it 





## If FFT is necessary for analysis:
Consider replaceing CG math with nalgebra glm 
Means we can use nalgebra with fft for those features.


Useful crates:
Units of measerment, for the pixel sizes 
2dfft, for doing image processing. 

If needed, consider swapping to glm::nalgebra.
Not the end of the world though, as we are able to do a lot 





## For research proposal:
More clear on what needs to be done by first stage review
Where are you, what is missing?
Document how close to schedule you are


Ask Rafael how to handle the FFT

How do we create this light field.

Do we do it as a render?


First stage:
Given the two panels, compute what would be in the panels, and reproduce what
the observer will be seen (what would a camera see)

Have some model off in the distance, have the two screens. 
Sample the light 

What should be placed on the two monitors, to see the box

Need to lay down and solve the "core problem"

Do a simple implementation for now.

**Action Points:**
Need to rethink plan, as there is the deliverable


First stage needs to be changed:
Week of the 15th May. 

Get to the simplest solution till the end. 

First stage:
I have a full pipeline. Very basic, but I have simple input and output.

First: Sample the light field, this is where the outline of the object is
Second: From that, make a function that solves for this

## Rafael meeting:

Change the approach: don't sample the light field through snapshots, instead
think of all possible rays that can be between both panels.
We can describe this as a 4D (x, y, x^prime, y^prime)

Look into how to change this to be WGPU? 
Or hack the GLSL implementation to give us this?

square of the pixel amount of possible light rays


Ray trace the scene but replace the white rays with the red rays!

## WGPU Check point:
Managed to rebuild progress on wgpu!
What is missing? 
System for changing uniforms. There are many uniforms that likely need to be recomputed and repased at redraw (Camera, movement)
Passing textures
Expand world!
Bringing in panels 

Move towards light distortion
Bring in the textures of the monkeys




Move Uniform binding out of main, break it up into much smaller things


QOL features:
These two build on the same system
Undo/redo (stack of app states?)
save state (to file, read from file)
Derive macro may be the way to go
6 degrees of motion for world

Other features:


Split matrix across work group.
Write to Buffer. Unpacking the buffer will be another job
Build sample matrix
sample the world as two vecs 





## Before first stage review:
Load textures of solving
Stick to 30 by 30 for now?

Yes

And get writing

Increase solver to be generic on scale!

Scale correctly

Writing should be done

GO to example that can be written by hand

4 by 1
4 by 4

Right now, there is a change the buffer is full of old readings. 

There is something wrong at small sizes. I suspect the data coming back is somewhat compromised
The borders are not good

Go back and build.


## After first stage

Change coloring
Done!

Experiment with average, max, different view points
Introduce units

Can you change the mapping?


