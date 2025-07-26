
# Project TODO  

## List  
- [x] slight ui rework (mainly just make the text not overflow)  
- [x] fly and general movement rework  
- [x] fov and general camera rework (also drag nice-ify)  
- [x] making the save-load work  
- [x] world gen from noise  
- [x] settings for sound volume and stuff  
- [x] refine inventory and item and block things  
- [x] 2d texture array and load in some other textures  
- [x] New UI element as Slider  
- [x] A "multi-state" button kind of thing  
- [x] chunk meshing correction - neighbors  


- [ ] item and block things correctly  
- [ ] making the save-load auto trigger (actually load the world on startup)  
- [ ] correcting the save to be world wise not just player pos relative 
- [ ] Ambient oc. and other lighting  
- [ ] making an extra optional label attach-able to most UI elements  
- [ ] hardcode the "missing texture" texture (purple black or idk)  
- [ ] Some basic Rounding for UI elements   
- [ ] Making a new "panel" kind of thing with scrollable contents  


- [ ] Maybe make the UIelements be centered around their pos  

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
[link](https://google.github.io/filament/Filament.md.html) - middle ?  
