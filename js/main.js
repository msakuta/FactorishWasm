import time from "../img/time.png";
import dirt from "../img/dirt.png";
import iron from "../img/iron.png";
import coal from "../img/coal.png";
import copper from "../img/copper.png";
import transport from "../img/transport.png";
import splitter from "../img/splitter.png";
import chest from "../img/chest.png";
import mine from "../img/mine.png";
import assembler from "../img/assembler.png";
import furnace from "../img/furnace.png";
import waterWell from "../img/waterwell.png";
import boiler from "../img/boiler.png";
import inserter from "../img/inserter-base.png";
import direction from "../img/direction.png";
import ore from "../img/ore.png";
import coalOre from "../img/coal-ore.png";
import copperOre from "../img/copper-ore.png";
import ironPlate from "../img/metal.png";
import steelPlate from "../img/steel-plate.png";
import copperPlate from "../img/copper-plate.png";
import copperWire from "../img/copper-wire.png";
import circuit from "../img/circuit.png";
import gear from "../img/gear.png";
import pipeItem from "../img/pipe-item.png";
import steamEngine from "../img/steam-engine.png";
import rotateImage from "../img/rotate.png";
import closeImage from "../img/close.png";
import rightarrow from "../img/rightarrow.png";


import { FactorishState } from "../pkg/index.js";

/// We may no longer need support for IE, since WebAssembly is not supported by IE anyway.
function isIE(){
    var ua = window.navigator.userAgent;
    var msie = ua.indexOf('MSIE ');
    var trident = ua.indexOf('Trident/');
    return msie > 0 || trident > 0;
}

(async function(){
    // We could fetch and await in Rust code, but it's far easier to do in JavaScript runtime.
    // We initiate promises at the very beginning of the initialization, and by the time we initialize everything
    // we should have bitmaps ready.
    let loadImages = [
        ["dirt", dirt],
        ["iron", iron],
        ["coal", coal],
        ["copper", copper],
        ["transport", transport],
        ["chest", chest],
        ["mine", mine],
        ["furnace", furnace],
        ["assembler", assembler],
        ["inserter", inserter],
        ["direction", direction],
        ["ore", ore],
        ["coalOre", coalOre],
        ["ironPlate", ironPlate],
        ["copperOre", copperOre],
        ["copperPlate", copperPlate],
        ["gear", gear],
    ].map(async ([name, src]) => {
        const res = await fetch(src);
        return [name, await createImageBitmap(await res.blob())];
    });

    let sim = new FactorishState(updateInventory);

    const canvas = document.getElementById('canvas');
    const canvasSize = canvas.getBoundingClientRect();
    const ctx = canvas.getContext('2d');
    const container = document.getElementById('container2');
    const containerRect = container.getBoundingClientRect();
    const inventoryElem = document.getElementById('inventory2');

    const infoElem = document.createElement('div');
    infoElem.style.position = 'absolute';
    infoElem.style.backgroundColor = '#ffff7f';
    infoElem.style.border = '1px solid #00f';
    container.appendChild(infoElem);

    var selectedInventory = null;

    const tilesize = 32;
    const textType = isIE() ? "Text" : "text/plain";
    var windowZIndex = 1000;
    const objViewSize = tilesize / 2; // View size is slightly greater than hit detection radius
    const tableMargin = 10.;
    const miniMapSize = 200;
    const miniMapElem = document.createElement('div');
    miniMapElem.style.position = 'absolute';
    miniMapElem.style.border = '1px solid #000';
    miniMapElem.onclick = function(evt){
        var rect = this.getBoundingClientRect();
        scrollPos[0] = Math.min(xsize - viewPortWidth - 1, Math.max(0, Math.floor((evt.clientX - rect.left) / rect.width * xsize - viewPortWidth / 2.)));
        scrollPos[1] = Math.min(ysize - viewPortHeight - 1, Math.max(0, Math.floor((evt.clientY - rect.top) / rect.height * ysize - viewPortHeight / 2.)));
        updateAllTiles();
    };
    container.appendChild(miniMapElem);
    miniMapElem.style.width = miniMapSize + 'px';
    miniMapElem.style.height = miniMapSize + 'px';
    miniMapElem.style.left = (canvasSize.right - containerRect.left + tableMargin) + 'px';
    miniMapElem.style.top = (canvasSize.top - containerRect.top) + 'px';
    const mrect = miniMapElem.getBoundingClientRect();

    infoElem.style.left = (canvasSize.right + tableMargin) + 'px';
    infoElem.style.top = (mrect.bottom - containerRect.top + tableMargin) + 'px';
    infoElem.style.width = miniMapSize + 'px';
    infoElem.style.height = (canvasSize.height - mrect.height - tableMargin) + 'px';
    infoElem.style.textAlign = 'left';

    var toolDefs = sim.tool_defs();
    var toolElems = [];
    var toolOverlays = [];
    var toolCursorElem;
    // Tool bar
    var toolBarElem = document.getElementById('toolBar');
    toolBarElem.style.borderStyle = 'solid';
    toolBarElem.style.borderWidth = '1px';
    toolBarElem.style.borderColor = 'red';
    toolBarElem.style.position = 'absolute';
    toolBarElem.margin = '3px';
    toolBarElem.style.top = '480px';
    toolBarElem.style.left = '50%';
    toolBarElem.style.width = ((toolDefs.length + 1) * tilesize + 8) + 'px';
    toolBarElem.style.height = (tilesize + 8) + 'px';
    var toolBarCanvases = [];
    for(var i = 0; i < toolDefs.length; i++){
        var toolContainer = document.createElement('span');
        toolContainer.style.position = 'absolute';
        toolContainer.style.display = 'inline-block';
        toolContainer.style.width = '31px';
        toolContainer.style.height = '31px';
        toolContainer.style.top = '4px';
        toolContainer.style.left = (32.0 * i + 4) + 'px';
        toolContainer.style.border = '1px black solid';

        // Overlay for item count
        var overlay = document.createElement('div');
        toolOverlays.push(overlay);
        overlay.setAttribute('class', 'overlay noselect');
        overlay.innerHTML = '0';

        var toolElem = document.createElement("canvas");
        toolElems.push(toolElem);
        toolElem.width = 32;
        toolElem.height = 32;
        toolElem.style.left = '0px';
        toolElem.style.top = '0px';
        toolElem.style.width = '31px';
        toolElem.style.height = '31px';
        toolElem.style.position = 'absolute';
        toolElem.style.textAlign = 'center';
        toolElem.onmousedown = function(e){
            var currentTool = toolElems.indexOf(this);
            var state = sim.select_tool(currentTool);
            if(!toolCursorElem){
                toolCursorElem = document.createElement('div');
                toolCursorElem.style.border = '2px blue solid';
                toolCursorElem.style.pointerEvents = 'none';
                toolBarElem.appendChild(toolCursorElem);
            }
            toolCursorElem.style.position = 'absolute';
            toolCursorElem.style.top = '4px';
            toolCursorElem.style.left = (tilesize * currentTool + 4) + 'px';
            toolCursorElem.style.width = '30px';
            toolCursorElem.style.height = '30px';
            toolCursorElem.style.display = state ? 'block' : 'none';
        }
        toolElem.onmouseenter = function(e){
            var idx = toolElems.indexOf(this);
            if(idx < 0 || toolDefs.length <= idx)
                return;
            var tool = toolDefs[idx];
            // var r = this.getBoundingClientRect();
            // var cr = container.getBoundingClientRect();
            // toolTip.style.left = (r.left - cr.left) + 'px';
            // toolTip.style.top = (r.bottom - cr.top) + 'px';
            // toolTip.style.display = 'block';
            // var desc = tool.prototype.toolDesc();
            // if(0 < desc.length)
            //     desc = '<br>' + desc;
            // toolTip.innerHTML = '<b>' + tool.prototype.name + '</b>' + desc;
        };
        toolElem.onmouseleave = function(e){
            // toolTip.style.display = 'none';
        };
        toolContainer.appendChild(toolElem);
        toolBarCanvases.push(toolElem);
        toolContainer.appendChild(overlay);
        toolBarElem.appendChild(toolContainer);
    }
    var rotateButton = document.createElement('div');
    rotateButton.style.width = '31px';
    rotateButton.style.height = '31px';
    rotateButton.style.position = 'relative';
    rotateButton.style.top = '4px';
    rotateButton.style.left = (32.0 * i + 4) + 'px';
    rotateButton.style.border = '1px blue solid';
    rotateButton.style.backgroundImage = `url(${rotateImage}`;
    rotateButton.onmousedown = function(e){
        rotate();
    }
    toolBarElem.appendChild(rotateButton);
    // Set the margin after contents are initialized
    toolBarElem.style.marginLeft = (-(toolBarElem.getBoundingClientRect().width + miniMapSize + tableMargin) / 2) + 'px';

    try{
        const loadedImages = await Promise.all(loadImages);
        sim.render_init(canvas, infoElem, loadedImages);
    } catch(e) {
        alert(`FactorishState.render_init failed: ${e}`);
    }

    updateToolBarImage();

    function updateToolBarImage(){
        for(var i = 0; i < toolBarCanvases.length; i++){
            var canvasElem = toolBarCanvases[i];
            var context = canvasElem.getContext('2d');
            sim.render_tool(i, context);
        }
    }

    function rotate(){
        var newRotation = sim.rotate_tool();
        updateToolBarImage();
    }

    function updateToolBar(){
        var inventory = sim.tool_inventory();
        for(var i = 0; i < inventory.length; i++)
            toolOverlays[i].innerHTML = inventory[i];
    }

    function getImageFile(type){
        switch(type){
        case 'time':
            return time;
        case 'Iron Ore':
            return ore;
        case 'Iron Plate':
            return ironPlate;
        case 'Steel Plate':
            return steelPlate;
        case 'Copper Ore':
            return copperOre;
        case 'Copper Plate':
            return copperPlate;
        case 'Coal Ore':
            return coalOre;
        case 'Gear':
            return gear;
        case 'Copper Wire':
            return copperWire;
        case 'Circuit':
            return circuit;
        case 'Transport Belt':
            return transport;
        case 'Splitter':
            return splitter;
        case 'Inserter':
            return [inserter, 2];
        case 'Chest':
            return chest;
        case 'Ore Mine':
            return mine;
        case 'Furnace':
            return [furnace, 3];
        case 'Assembler':
            return assembler;
        case 'Water Well':
            return waterWell;
        case 'Boiler':
            return [boiler, 3];
        case 'Pipe':
            return pipeItem;
        case 'SteamEngine':
            return steamEngine;
        default:
            return "";
        }
    }

    function updateInventory(inventory){
        updateInventoryInt(playerInventoryElem, sim, false, inventory);
    }

    function updateStructureInventory(pos){
        if(pos){
            // Don't update with non-selected structure inventory
            const selPos = sim.get_selected_inventory();
            if(!selPos || pos[0] !== selPos[0] || pos[1] !== selPos[1])
                return;
        }
        updateInventoryInt(inventoryContentElem, sim, false, sim.get_structure_inventory(
            ...(pos ? pos : sim.get_selected_inventory())));
    }

    function generateItemImage(i, iconSize, count){
        var img = document.createElement('div');
        var imageFile = getImageFile(i);
        img.style.backgroundImage = 'url(' + (imageFile instanceof Array ?
            imageFile[0] : imageFile) + ')';
        var size = iconSize ? 32 : objViewSize;
        img.style.width = size + 'px';
        img.style.height = size + 'px';
        img.style.display = 'inline-block';
        if(imageFile instanceof Array)
            img.style.backgroundSize = size * imageFile[1] + 'px ' + size + 'px';
        else
            img.style.backgroundSize = size + 'px ' + size + 'px';
        img.setAttribute('draggable', 'false');
        if(iconSize && count){
            var container = document.createElement('span');
            container.style.position = 'relative';
            container.style.display = 'inline-block';
            container.style.width = size + 'px';
            container.style.height = size + 'px';
            container.appendChild(img);
            var overlay = document.createElement('div');
            overlay.setAttribute('class', 'overlay noselect');
            overlay.innerHTML = count;
            container.appendChild(overlay);
            return container;
        }
        return img;
    }

    function updateInventoryInt(elem, owner, icons, [inventory, selectedInventoryItem]){
        // Local function to update DOM elements based on selection
        function updateInventorySelection(elem){
            for(var i = 0; i < elem.children.length; i++){
                var celem = elem.children[i];
                celem.style.backgroundColor =
                    celem.itemName === selectedInventoryItem ? "#00ffff" : "";
            }
        }
    
        // Clear the elements first
        while(elem.firstChild)
            elem.removeChild(elem.firstChild);

        for(var i in inventory){
            var [name, v] = inventory[i];
            var div;
            if(icons){
                div = generateItemImage(name, true, v);
            }
            else{
                div = document.createElement('div');
                div.appendChild(generateItemImage(name));
                var text = document.createElement('span');
                text.innerHTML = v + ' ' + name;
                div.appendChild(text);
                div.style.textAlign = 'left';
            }
            if(selectedInventory === owner && selectedInventoryItem === name)
                div.style.backgroundColor = '#00ffff';
            div.setAttribute('class', 'noselect');
            div.itemName = name;
            div.itemAmount = v;
            /// Either clicking or start dragging will select the item, so that
            /// it can be moved on drop
            function selectThisItem(itemName){
                selectedInventory = owner;
                selectedInventoryItem = itemName;
                if(elem === playerInventoryElem){
                    sim.select_player_inventory(selectedInventoryItem);
                }
                else{
                    sim.select_structure_inventory(selectedInventoryItem);
                }
                updateInventorySelection(elem);
            };
            div.onclick = function(evt){
                selectThisItem(this.itemName);
                evt.stopPropagation();
            };
            div.setAttribute('draggable', 'true');
            div.ondragstart = function(ev){
                console.log("dragStart");
                selectThisItem(this.itemName);
                ev.dataTransfer.dropEffect = 'move';
                // Encode information to determine item to drop into a JSON
                ev.dataTransfer.setData(textType, JSON.stringify(
                    {type: ev.target.itemName, fromPlayer: elem === playerInventoryElem}));
            };
            elem.appendChild(div);
        }
    }

    inventoryElem.ondragover = function(ev){
        var ok = false;
        for(var i = 0; i < ev.dataTransfer.types.length; i++){
            if(ev.dataTransfer.types[i].toUpperCase() === textType.toUpperCase())
                ok = true;
        }
        if(ok){
            ev.preventDefault();
            // Set the dropEffect to move
            ev.dataTransfer.dropEffect = "move";
        }
    }
    inventoryElem.ondrop = function(ev){
        ev.preventDefault();
        var data = JSON.parse(ev.dataTransfer.getData(textType));
        if(data.fromPlayer){
            // The amount could have changed during dragging, so we'll query current value
            // from the source inventory.
            if(sim.move_selected_inventory_item(!data.fromPlayer)){
                updateInventory(sim.get_player_inventory());
                updateToolBar();
                updateStructureInventory();
            }
        }
    }
    inventoryElem.style.display = 'none';

    const inventoryContentElem = document.getElementById('inventory2Content');
    inventoryContentElem.onclick = function(){
        onInventoryClick(false);
    };

    const inventory2CloseButton = document.getElementById("inventory2CloseButton");
    inventory2CloseButton.style.backgroundImage = `url(${closeImage})`;
    inventory2CloseButton.addEventListener("click", function(){
        inventoryElem.style.display = "none";
    });

    function dragWindowMouseDown(evt,elem,pos){
        pos = [evt.screenX, evt.screenY];
        bringToTop(elem);
        var mousecaptorElem = document.getElementById('mousecaptor');
        mousecaptorElem.style.display = 'block';

        // Dragging moves windows
        function mousemove(evt){
            if(!pos)
                return;
            var containerElem = document.getElementById('container2');
            var cr = containerElem.getBoundingClientRect();
            var rel = [evt.screenX - pos[0], evt.screenY - pos[1]];
            pos = [evt.screenX, evt.screenY];
            var r = elem.getBoundingClientRect();
            var left = elem.style.left !== '' ? parseInt(elem.style.left) : (cr.left + cr.right) / 2;
            var top = elem.style.top !== '' ? parseInt(elem.style.top) : (cr.top + cr.bottom) / 2;
            elem.style.left = (left + rel[0]) + 'px';
            elem.style.top = (top + rel[1]) + 'px';
        }
        
        mousecaptorElem.addEventListener('mousemove', mousemove);
        mousecaptorElem.addEventListener('mouseup', function(evt){
            // Stop dragging a window
            elem = null;
            this.removeEventListener('mousemove', mousemove);
            this.style.display = 'none';
        });
    }

    /// An array of window elements which holds order of z indices.
    var windowOrder = [];

    var inventoryDragStart = null;

    var inventoryTitleElem = document.getElementById('inventory2Title');

    placeCenter(inventoryElem);
    windowOrder.push(inventoryElem);

    inventoryTitleElem.addEventListener('mousedown', function(evt){
        dragWindowMouseDown(evt, inventoryElem, inventoryDragStart);
    });

    /// Bring a window to the top on the other windows.
    function bringToTop(elem){
        var oldIdx = windowOrder.indexOf(elem);
        if(0 <= oldIdx && oldIdx < windowOrder.length - 1){
            windowOrder.splice(oldIdx, 1);
            windowOrder.push(elem);
            for(var i = 0; i < windowOrder.length; i++)
                windowOrder[i].style.zIndex = i + windowZIndex;
        }
        var mousecaptorElem = document.getElementById('mousecaptor');
        mousecaptorElem.style.zIndex = i + windowZIndex; // The mouse capture element comes on top of all other windows
    }

    function showInventory(c, r){
        if(inventoryElem.style.display !== "none"){
            inventoryElem.style.display = "none";
            return;
        }
        // else if(tile.structure && tile.structure.inventory){
        else{
            inventoryElem.style.display = "block";
            bringToTop(inventoryElem);
            // var recipeSelectButtonElem = document.getElementById('recipeSelectButton');
            // recipeSelectButtonElem.style.display = !inventoryTarget.recipes ? "none" : "block";
            // toolTip.style.display = "none"; // Hide the tool tip for "Click to oepn inventory"
            updateInventoryInt(inventoryContentElem, sim, false, sim.get_structure_inventory(c, r));
        }
        // else{
        //     inventoryContent.innerHTML = "";
        // }
    }

    let recipeTarget = null;

    function recipeDraw(recipe, onclick){
        var ret = "";
        ret += "<div class='recipe-box'"
            + (onclick ? " onclick='" + onclick + "'" : "") + ">";
        ret += "<span style='display: inline-block; margin: 1px'>" +
            getHTML(generateItemImage("time", true, recipe.recipe_time), true) + "</span>";
        ret += "<span style='display: inline-block; width: 50%'>";
        for(var k in recipe.input)
            ret += getHTML(generateItemImage(k, true, recipe.input[k]), true);
        ret += `</span><img src='${rightarrow}' style='width: 20px; height: 32px'><span style='display: inline-block; width: 10%'>`;
        for(var k in recipe.output)
            ret += getHTML(generateItemImage(k, true, recipe.output[k]), true);
        ret += "</span></div>";
        return ret;
    }

    /// Convert a HTML element to string.
    /// If deep === true, descendants are serialized, too.
    function getHTML(who, deep){
        var div = document.createElement('div');
        div.appendChild(who.cloneNode(false));
        var txt = div.innerHTML;
        if(deep){
            var ax = txt.indexOf('>')+1;
            txt= txt.substring(0, ax)+who.innerHTML+ txt.substring(ax);
        }
        return txt;
    }

    function showRecipeSelect(){
        var recipeSelector = document.getElementById('recipeSelector');
        var recipeSelectorContent = document.getElementById('recipeSelectorContent');
        if(recipeSelector.style.display !== "none"){
            recipeSelector.style.display = "none";
            return;
        }
        else if(sim.get_selected_inventory()){
            recipeSelector.style.display = "block";
            bringToTop(recipeSelector);
            recipeTarget = sim.get_selected_inventory();
            var text = "";
            var recipes = sim.get_structure_recipes(...sim.get_selected_inventory());
            for(var i = 0; i < recipes.length; i++)
                text += recipeDraw(recipes[i], "Conveyor.recipeSelectClick(" + i + ")");
            recipeSelectorContent.innerHTML = text;
        }
        else{
            recipeTarget = null;
            recipeSelectorContent.innerHTML = "No recipes available";
        }
    }
    
    function hideRecipeSelect(){
        var recipeSelector = document.getElementById('recipeSelector');
        recipeSelector.style.display = "none";
    }
    const recipeSelectorCloseButton = document.getElementById('recipeSelectorCloseButton');
    recipeSelectorCloseButton.onclick = hideRecipeSelect;
    recipeSelectorCloseButton.style.backgroundImage = `url(${closeImage})`;

    // Place a window element at the center of the container, assumes the windows have margin set in the middle.
    function placeCenter(elem){
        var containerElem = document.getElementById('container2');
        var cr = containerElem.getBoundingClientRect();
        elem.style.left = ((cr.left + cr.right) / 2) + 'px';
        elem.style.top = ((cr.top + cr.bottom) / 2) + 'px';
    }

    placeCenter(inventoryElem);
    windowOrder.push(inventoryElem);

    const recipeSelectButtonElem = document.getElementById('recipeSelectButton');
    recipeSelectButtonElem.onclick = showRecipeSelect;

    const playerElem = document.createElement('div');
    playerElem.style.overflow = 'visible';
    playerElem.style.borderStyle = 'solid';
    playerElem.style.borderWidth = '1px';
    playerElem.style.border = '1px solid #00f';
    playerElem.style.backgroundColor = '#ffff7f';
    playerElem.style.position = 'absolute';
    playerElem.style.margin = '3px';
    playerElem.style.left = '50%';
    playerElem.style.top = '520px';
    playerElem.style.width = (320) + 'px';
    playerElem.style.height = (160) + 'px';
    container.appendChild(playerElem);
    playerElem.style.marginLeft = (-(playerElem.getBoundingClientRect().width + miniMapSize + tableMargin) / 2) + 'px';

    const playerInventoryElem = document.createElement('div');
    playerInventoryElem.style.position = 'relative';
    playerInventoryElem.style.overflowY = 'scroll';
    playerInventoryElem.style.width = '100%';
    playerInventoryElem.style.height = '100%';
    playerInventoryElem.style.textAlign = 'left';
    playerInventoryElem.ondragover = function(ev){
        var ok = false;
        for(var i = 0; i < ev.dataTransfer.types.length; i++){
            if(ev.dataTransfer.types[i].toUpperCase() === textType.toUpperCase())
                ok = true;
        }
        if(ok){
            ev.preventDefault();
            // Set the dropEffect to move
            ev.dataTransfer.dropEffect = "move";
        }
    }
    playerInventoryElem.ondrop = function(ev){
        ev.preventDefault();
        var data = JSON.parse(ev.dataTransfer.getData(textType));
        if(!data.fromPlayer){
            if(sim.move_selected_inventory_item(!data.fromPlayer)){
                updateInventory(sim.get_player_inventory());
                updateToolBar();
                updateStructureInventory();
            }
        }
    }
    playerInventoryElem.onclick = function(){onInventoryClick(true)};
    playerElem.appendChild(playerInventoryElem);

    function onInventoryClick(isPlayer){
        // Update only if the selected inventory is the other one from destination.
        if(sim.get_selected_inventory() !== null){
            if(sim.move_selected_inventory_item(isPlayer)){
                updateInventory(sim.get_player_inventory());
                updateToolBar();
                updateStructureInventory();
            }
        }
    }

    canvas.addEventListener("mousedown", function(evt){
        processEvents(sim.mouse_down([evt.offsetX, evt.offsetY], evt.button));
        evt.stopPropagation();
        evt.preventDefault();
        return false;
    });
    canvas.addEventListener("contextmenu", function(evt){
        evt.preventDefault();
    });
    canvas.addEventListener("mousemove", function(evt){
        sim.mouse_move([evt.offsetX, evt.offsetY]);
    });

    canvas.addEventListener("mouseleave", function(evt){
        sim.mouse_leave([evt.offsetX, evt.offsetY]);
    });

    function onKeyDown(event){
        if(sim.on_key_down(event.keyCode))
            updateToolBarImage();
    }
    window.addEventListener( 'keydown', onKeyDown, false );

    updateToolBar();

    updateInventory(sim.get_player_inventory());

    function processEvents(events){
        if(!events)
            return;
        for(let event of events){
            if(event[0] === "updateStructureInventory"){
                console.log(`updateStructureInventory event received ${event}`);
                updateStructureInventory([event[1], event[2]]);
            }
            if(event[0] === "updatePlayerInventory"){
                console.log("updatePlayerInventory event received");
                updateInventory(sim.get_player_inventory());
                updateToolBar();
            }
            else if(event[0] === "showInventory"){
                const [_command, x, y] = event;
                showInventory(x, y);
            }
        }
    }

    window.setInterval(function(){
        processEvents(sim.simulate(0.05));
        let result = sim.render(ctx);
        // console.log(result);
    }, 50);
    // simulate()
})();
