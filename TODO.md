
# Project TODO  

## High Priority  
- [ ] slight ui rework (mainly just make the text not overflow) 
- [ ] settings for sound volume font and stuff 
- [ ] fly and general movement rework 
- [ ] fov and general camera rework (also drag nice-ify) 
- [ ] making the save-load actually do stuff 

## Low Priority  
- [ ] hardcode the "missing texture" texture (purple black or idk) 
- [ ] 2d texture array and load in some other textures -> more blocks 
- [ ] New UI element as Slider 
- [ ] Some basic Rounding for UI elements 
- [ ] Maybe a "multi-state" button kind of thing (like a slider but with a button) 
- [ ] Making a new "panel" kind of thing with scrollable contents 


## Compiled stuff

Technique	Purpose	Example 	Usage  
Blinn-Phong	Cheap lighting		Wooden crates, character skin  
SMAA/FXAA	Smooth edges		Post-process pass  
Baked Lighting	Static shadows		Buildings, terrain  
Texture Atlases	Reduce draw calls	Foliage, debris  
Fog		Hide LOD transitions	Dungeons, open worlds  
 
## Textures (for realistic lights and heights)  
Albedo (RGBA) – Base texture.  
Normal (three channel).  
Bump (one channel).  
Roughness (one channel).  
Metallic (one channel).  

## Actual lighting (algorythms)  
Physically Based Rendering (PBR) (currently the "best", actually physics based)  
Blinn-Phong (BP) (one of the earliest from late 20th century, pretty cheap too)  
https://google.github.io/filament/Filament.md.html - middle ?  
