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

Inspect 3D world? does that make any sense?

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

Extend raytracer to use textures
Extend raytracer to deal with semi opaquee object
Extend ray tracer to be concurrent?






