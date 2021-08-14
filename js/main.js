import rotateImage from "../img/rotate.png";
import closeImage from "../img/close.png";
import rightarrow from "../img/rightarrow.png";
import fuelBack from "../img/fuel-back.png";
import inventory from "../img/inventory.png";

import { loadImages, getImageFile } from "./images.js";
import { FactorishState } from "../pkg/index.js";

/// We may no longer need support for IE, since WebAssembly is not supported by IE anyway.
function isIE(){
    var ua = window.navigator.userAgent;
    var msie = ua.indexOf('MSIE ');
    var trident = ua.indexOf('Trident/');
    return msie > 0 || trident > 0;
}

const tooltipZIndex = 10000;
let xsize = 128;
let ysize = 128;
let unlimited = true;

(async function(){

    function sliderInit(sliderId, labelId, writer, logarithmic=false){
        const slider = document.getElementById(sliderId);
        const label = document.getElementById(labelId);
        label.innerHTML = slider.value;
    
        const sliderStats = () => {
          const [minp, maxp] = ["min", "max"].map(attr => parseFloat(slider.getAttribute(attr)));
          if(minp <= 0){
            throw "Minimum value for logarithmic slider must not be 0";
          }
          const [minv, maxv] = [minp, maxp].map(Math.log);
          // calculate adjustment factor
          const scale = (maxv-minv) / (maxp-minp);
          return {minp, maxp, minv, maxv, scale};
        };
    
        const updateFromInput = (_event) => {
          let value;
          if(logarithmic){
            const {minp, minv, scale} = sliderStats();
            value = Math.exp(minv + scale*(parseFloat(slider.value) - minp));
            label.innerHTML = value.toFixed(8);
          }
          else{
            value = parseFloat(slider.value);
            label.innerHTML = value;
          }
          writer(value);
        }
        const updateFromValue = (value) => {
          if(logarithmic){
            const {minp, scale, minv} = sliderStats();
    
            // Inverse of updateFromInput
            slider.value = (Math.log(value) - minv) / scale + minp;
            label.innerHTML = value.toFixed(8);
          }
          else{
            slider.value = value;
            label.innerHTML = value;
          }
          writer(value);
        };
        if(logarithmic){
          // Update the UI according to logarithmic scale even before the user touches the slider
          updateFromValue(parseFloat(slider.value));
        }
        slider.addEventListener("input", updateFromInput);
        return {elem: slider, update: updateFromValue};
    }

    let terrainSeed = 8913095;
    const seedElem = document.getElementById("seed");
    if(seedElem){
        seedElem.value = terrainSeed;
        seedElem.addEventListener("input", _ => {
            terrainSeed = parseInt(seedElem.value)
        });
    }

    let waterNoiseThreshold = 0.28;
    sliderInit("waterNoiseThreshold", "waterNoiseThresholdLabel", value => waterNoiseThreshold = value);
    let resourceAmount = 1000.;
    sliderInit("resourceAmount", "resourceAmountLabel", value => resourceAmount = value);
    let noiseScale = 5.;
    sliderInit("noiseScale", "noiseScaleLabel", value => noiseScale = value);
    let noiseThreshold = 0.30;
    sliderInit("noiseThreshold", "noiseThresholdLabel", value => noiseThreshold = value);
    let noiseOctaves = 3;
    sliderInit("noiseOctaves", "noiseOctavesLabel", value => noiseOctaves = value);

    function initPane(buttonId, containerId){
        const button = document.getElementById(buttonId);
        const container = document.getElementById(containerId);
        if(button){
            button.addEventListener("click", (event) => {
                container.style.display = container.style.display === "none" ? "block" : "none";
            });
        }
    }
    initPane("paramsButton", "paramsContainer");
    initPane("viewButton", "viewContainer");

    const scenarioSelectElem = document.getElementById("scenarioSelect");

    let paused = false;

    const canvas = document.getElementById('canvas');
    const popupContainer = document.getElementById("popupContainer");
    let canvasSize = canvas.getBoundingClientRect();
    const context = canvas.getContext('webgl', { alpha: false });

    const container = document.getElementById('container2');
    const containerRect = container.getBoundingClientRect();
    const inventoryElem = document.getElementById('inventory2');
    const mouseIcon = document.getElementById("mouseIcon");

    const toolTip = document.createElement('dim');
    toolTip.setAttribute('id', 'tooltip');
    toolTip.setAttribute('class', 'noselect');
    toolTip.innerHTML = 'hello there';
    toolTip.style.zIndex = tooltipZIndex; // Usually comes on top of all the other elements
    toolTip.style.display = 'none'; // Initially invisible
    container.appendChild(toolTip);

    const infoElem = document.createElement('div');
    infoElem.style.position = 'absolute';
    infoElem.style.backgroundColor = 'rgba(255, 255, 191, 0.75)';
    infoElem.style.border = '1px solid #00f';
    container.appendChild(infoElem);


    let loadedImages;
    let sim;
    try{
        loadedImages = await Promise.all(loadImages);

        sim = new FactorishState(
        {
            width: xsize,
            height: ysize,
            unlimited,
            terrain_seed: terrainSeed,
            water_noise_threshold: waterNoiseThreshold,
            resource_amount: resourceAmount,
            noise_scale: noiseScale,
            noise_threshold: noiseThreshold,
            noise_octaves: noiseOctaves,
        },
        updateInventory,
        popupText,
        scenarioSelectElem.value,
        context,
        loadedImages,
        );

        sim.render_init(canvas, infoElem, loadedImages);
        sim.render_gl_init(context);
    } catch(e) {
        alert(`FactorishState.render_init failed: ${e}`);
    }

    const refreshSize = (event) => {
        canvasSize = canvas.getBoundingClientRect();
        canvas.width = canvasSize.width;
        canvas.height = canvasSize.height;
        popupContainer.style.width = `${canvasSize.width}px`;
        popupContainer.style.height = `${canvasSize.height}px`;
        context.viewport(0, 0, canvas.width, canvas.height);
        infoElem.style.height = (canvasSize.height - mrect.height - tableMargin * 3) + 'px';
        sim.reset_viewport(canvas);
    };
    document.body.onresize = refreshSize;
    const headerButton = document.getElementById("headerButton");
    const headerContainer = document.getElementById("headerContainer");

    function setHeaderVisible(v = "toggle"){
        if(v === "toggle"){
            v = headerContainer.style.display === "none";
        }
        headerContainer.style.display = v ? "block" : "none";
        headerButton.classList = "headerButton " + (v ? "open" : "");
        headerButton.innerHTML = v ? "^" : ""
    }

    if(headerButton){
        headerButton.addEventListener("click", () => setHeaderVisible());
        const viewSettings = JSON.parse(localStorage.getItem("FactorishWasmViewSettings"));
        // Default visible
        if(viewSettings && !viewSettings.headerVisible)
            setHeaderVisible(false);
    }

    let selectedInventory = null;
    let selectedInventoryItem = null;

    let miniMapDrag = null;
    const tilesize = 32;
    const textType = isIE() ? "Text" : "text/plain";
    var windowZIndex = 1000;
    const objViewSize = tilesize / 2; // View size is slightly greater than hit detection radius
    const tableMargin = 10.;
    const miniMapSize = 200;
    const miniMapElem = document.createElement('canvas');
    miniMapElem.style.position = 'absolute';
    miniMapElem.style.border = '1px solid #000';
    miniMapElem.onmousedown = (evt) => {
        miniMapDrag = [evt.offsetX, evt.offsetY];
    };
    miniMapElem.onmousemove = function(evt){
        if(miniMapDrag){
            sim.delta_viewport_pos(
                (evt.offsetX - miniMapDrag[0]) * tilesize,
                (evt.offsetY - miniMapDrag[1]) * tilesize,
                false);
            miniMapDrag = [evt.offsetX, evt.offsetY, true];
        }
    };
    miniMapElem.onmouseup = (evt) => miniMapDrag = false;
    miniMapElem.onmouseleave = (evt) => miniMapDrag = false;
    container.appendChild(miniMapElem);
    miniMapElem.setAttribute("width", miniMapSize);
    miniMapElem.setAttribute("height", miniMapSize);
    miniMapElem.style.width = miniMapSize + 'px';
    miniMapElem.style.height = miniMapSize + 'px';
    miniMapElem.style.right = '8px';
    miniMapElem.style.top = '8px';
    const mrect = miniMapElem.getBoundingClientRect();
    const miniMapContext = miniMapElem.getContext('2d');

    infoElem.style.right = '8px';
    infoElem.style.top = (mrect.bottom - containerRect.top + tableMargin) + 'px';
    infoElem.style.width = miniMapSize + 'px';

    infoElem.style.textAlign = 'left';

    const perfWidth = 200;
    const perfHeight = 200;
    const perfElem = document.createElement('canvas');
    perfElem.style.position = 'absolute';
    perfElem.style.pointerEvents = "none";
    perfElem.style.border = '1px solid #000';
    perfElem.setAttribute("width", perfWidth);
    perfElem.setAttribute("height", perfHeight);
    perfElem.style.width = perfWidth + 'px';
    perfElem.style.height = perfHeight + 'px';
    perfElem.style.left = '8px';
    perfElem.style.bottom = '8px';
    perfElem.style.backgroundColor = "rgba(0, 0, 0, 0.5)";
    container.appendChild(perfElem);
    const perfContext = perfElem.getContext('2d');

    const perfLabel = document.createElement('div');
    perfLabel.style.position = 'absolute';
    perfLabel.style.pointerEvents = "none";
    perfLabel.style.textAlign = "justify";
    perfLabel.style.left = '8px';
    perfLabel.style.bottom = '216px';
    perfLabel.style.padding = "4px";
    perfLabel.style.backgroundColor = "rgba(0, 0, 0, 0.75)";
    container.appendChild(perfLabel);

    refreshSize();

    const toolBeltSize = 10;
    var toolElems = [];
    var toolOverlays = [];
    var toolCursorElem;

    function updateToolCursor(){
        var currentTool = sim.get_selected_tool();
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
        toolCursorElem.style.display = currentTool !== null ? 'block' : 'none';
        updateMouseIcon();
    }

    function updateMouseIcon(){
        const item = sim.get_selected_tool_or_item();
        if(item){
            mouseIcon.style.display = "block";
            let imageFile = getImageFile(item);
            mouseIcon.style.backgroundImage = `url(${imageFile.url})`;
        }
        else
            mouseIcon.style.display = "none";
    }

    function setToolTip(elem, text){
        var r = elem.getBoundingClientRect();
        var cr = container.getBoundingClientRect();
        toolTip.style.display = 'block';
        toolTip.innerHTML = text;
        const toolTipRect = toolTip.getBoundingClientRect();
        toolTip.style.left = (r.left - cr.left) + 'px';
        toolTip.style.top = (r.top - cr.top - toolTipRect.height) + 'px';
    }
    const renderToolTip = (elem, idx) => {
        const tool = sim.get_tool_desc(idx);
        let text = "";
        if(!tool){
            text = "<b>Empty slot</b><br>"
                + "Select an item in the inventory and click here to put the item into this slot.";
        }
        else{
            var desc = tool[1];
            if(0 < desc.length)
                desc = '<br>' + desc;
            text = '<b>' + tool[0] + '</b>'
                + `<br><i>Shortcut: '${(idx + 1) % 10}'</i>` + desc;
        }
        setToolTip(elem, text);
    };

    function deselectPlayerInventory(){
        selectedInventory = null;
        sim.deselect_player_inventory();
        mouseIcon.style.display = "none";
    }

    // Tool bar
    var toolBarElem = document.getElementById('toolBar');
    toolBarElem.style.borderStyle = 'solid';
    toolBarElem.style.borderWidth = '1px';
    toolBarElem.style.borderColor = 'red';
    toolBarElem.style.position = 'absolute';
    toolBarElem.margin = '3px';
    // toolBarElem.style.top = '480px';
    // toolBarElem.style.left = '50%';
    toolBarElem.style.width = ((toolBeltSize + 2) * tilesize + 8) + 'px';
    toolBarElem.style.height = (tilesize + 8) + 'px';
    var toolBarCanvases = [];
    for(var i = 0; i < toolBeltSize; i++){
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
            const result = sim.select_tool(currentTool);
            if(result === "ShowInventory"){
                showInventory();
            }
            else{
                updateToolBarImage();
                updateToolBar();
                renderToolTip(this, currentTool);
            }
            updateInventory(sim.get_player_inventory());
            updateToolCursor(currentTool);
        }
        toolElem.onmouseenter = function(e){
            var idx = toolElems.indexOf(this);
            if(idx < 0 || toolBeltSize <= idx)
                return;
            renderToolTip(this, idx);
        };
        toolElem.onmouseleave = (_e) => toolTip.style.display = 'none';
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
    rotateButton.style.left = (32.0 * i++ + 4) + 'px';
    rotateButton.style.border = '1px blue solid';
    rotateButton.style.backgroundImage = `url(${rotateImage}`;
    rotateButton.onmousedown = () => rotate();
    rotateButton.onmouseenter = (e) => setToolTip(e.target, "<b>Rotate</b><br><i>Shortcut: (R)</i>");
    rotateButton.onmouseleave = (_e) => toolTip.style.display = 'none';
    toolBarElem.appendChild(rotateButton);
    // Set the margin after contents are initialized
    // toolBarElem.style.marginLeft = (-(toolBarElem.getBoundingClientRect().width + miniMapSize + tableMargin) / 2) + 'px';

    const inventoryButton = document.createElement('div');
    inventoryButton.style.width = '31px';
    inventoryButton.style.height = '31px';
    inventoryButton.style.position = 'absolute';
    inventoryButton.style.top = '4px';
    inventoryButton.style.left = (32.0 * i + 4) + 'px';
    inventoryButton.style.border = '1px blue solid';
    inventoryButton.style.backgroundImage = `url(${inventory})`;
    inventoryButton.onmousedown = () => showInventory();
    inventoryButton.onmouseenter = (e) => setToolTip(e.target, "<b>Inventory</b><br><i>Shortcut: (E)</i>");
    inventoryButton.onmouseleave = () => toolTip.style.display = 'none';
    toolBarElem.appendChild(inventoryButton);

    function updateToolBarImage(){
        for(var i = 0; i < toolBarCanvases.length; i++){
            var canvasElem = toolBarCanvases[i];
            var context = canvasElem.getContext('2d');
            try{
                sim.render_tool(i, context);
            } catch(e) {
                console.error(e);
            }
        }
    }

    function rotate(){
        if(sim.rotate_tool())
            updateToolBarImage();
    }

    function updateToolBar(){
        var inventory = sim.tool_inventory();
        for(var i = 0; i < inventory.length; i++)
            toolOverlays[i].innerHTML = inventory[i];
    }


    const POPUP_LIFE = 30;
    const popupTexts = [];
    function popupText(text, x, y){
        const elem = document.createElement("div");
        elem.className = "popupText";
        elem.style.left = `${x}px`;
        elem.style.top = `${y}px`;
        elem.innerHTML = text;
        popupContainer.appendChild(elem);
        popupTexts.push({
            elem,
            y,
            life: POPUP_LIFE,
        });
    }

    function animatePopupTexts(){
        for(let i = 0; i < popupTexts.length;) {
            const popup = popupTexts[i];
            popup.y -= 1;
            popup.elem.style.top = `${popup.y}px`;
            if(--popup.life <= 0){
                popupContainer.removeChild(popup.elem);
                popupTexts.splice(i, 1);
            }
            else{
                i++;
            }
        }
    }

    function updateInventory(inventory){
        try{
            updateInventoryInt(playerInventoryElem, sim, false, inventory);
        }catch(e){
            console.log(e);
        }
    }

    function updateStructureInventory(pos){
        if(pos){
            // Don't update with non-selected structure inventory
            const selPos = sim.get_selected_inventory();
            if(!selPos || pos[0] !== selPos[0] || pos[1] !== selPos[1])
                return;
        }
        const position = pos ? pos : sim.get_selected_inventory();
        updateInventoryInt(inventoryContentElem, sim, false, sim.get_structure_inventory(
            ...position, "Input"));
        updateInventoryInt(outputInventoryContentElem, sim, false, sim.get_structure_inventory(
            ...position, "Output"));
    }

    function generateItemImage(i, iconSize, count){
        var img = document.createElement('div');
        var imageFile = getImageFile(i);
        img.style.backgroundImage = `url(${imageFile.url})`;
        var size = iconSize ? 32 : objViewSize;
        img.style.width = size + 'px';
        img.style.height = size + 'px';
        img.style.display = 'inline-block';
        img.style.backgroundSize = size * imageFile.widthFactor + 'px ' + size * imageFile.heightFactor + 'px';
        img.setAttribute('draggable', 'false');
        if(iconSize){
            var container = document.createElement('span');
            container.style.position = 'relative';
            container.style.display = 'inline-block';
            container.style.width = size + 'px';
            container.style.height = size + 'px';
            container.appendChild(img);
            var overlay = document.createElement('div');
            overlay.setAttribute('class', 'overlay noselect');
            overlay.innerHTML = count || 0;
            container.appendChild(overlay);
            return container;
        }
        return img;
    }

    function microTask(f){
        Promise.resolve().then(f);
    }

    function updateInventoryInt(elem, owner, icons, inventoryData, titleElem = null){
        // Local function to update DOM elements based on selection
        function updateInventorySelection(elem){
            for(var i = 0; i < elem.children.length; i++){
                var celem = elem.children[i];
                celem.style.backgroundColor =
                    celem.itemName === selectedInventoryItem ? "#00ffff" : "";
            }
        }

        // Defer execution of updateMouseIcon in order to avoid 
        // "recursive use of an object detected which would lead to unsafe aliasing in rust"
        microTask(updateMouseIcon);

        if(!inventoryData || inventoryData.length === 0){
            elem.style.display = "none";
            if(titleElem)
                titleElem.style.display = "none";
            return;
        }
        elem.style.display = "block";
        if(titleElem)
            titleElem.style.display = "block";
        const [inventory, item] = inventoryData;

        selectedInventoryItem = item;

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
                if(selectedInventory === owner && selectedInventoryItem === itemName){
                    deselectPlayerInventory();
                    selectedInventoryItem = null;
                    updateInventorySelection(elem);
                    return;
                }
                selectedInventory = owner;
                selectedInventoryItem = itemName;
                if(elem === playerInventoryElem){
                    sim.select_player_inventory(selectedInventoryItem);
                    updateMouseIcon();
                }
                else{
                    sim.select_structure_inventory(selectedInventoryItem);
                }
                updateInventorySelection(elem);
            };
            div.onclick = (name => evt => {
                selectThisItem(name);
                evt.stopPropagation();
            })(name);
            div.setAttribute('draggable', 'true');
            div.ondragstart = (name => ev => {
                console.log("dragStart");
                selectThisItem(name);
                ev.dataTransfer.dropEffect = 'move';
                // Encode information to determine item to drop into a JSON
                ev.dataTransfer.setData(textType, JSON.stringify({
                    type: name,
                    fromPlayer: elem === playerInventoryElem,
                    inventoryType: elem === inventoryContentElem ? "Input" : "Output",
                }));
            })(name);
            elem.appendChild(div);
        }
    }

    const inventory2ClientElem = document.getElementById('inventory2Client');
    const inputInventoryTitleElem = document.getElementById('inputInventoryTitle');
    const inventoryContentElem = document.getElementById('inputInventoryContent');
    inventoryContentElem.onclick = () => onInventoryClick(false, true);
    const outputInventoryContentElem = document.getElementById('outputInventoryContent');
    outputInventoryContentElem.onclick = () => onInventoryClick(false, false);
    const outputInventoryTitleElem = document.getElementById('outputInventoryTitle');
    const burnerContainer = document.getElementById('burnerContainer');
    const inputFuelElem = document.getElementById('inputFuel');
    inputFuelElem.style.backgroundImage = `url(${fuelBack})`;

    [inventoryContentElem, outputInventoryContentElem, inputFuelElem].forEach((elem, idx) => {
        elem.ondragover = function(ev){
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
        elem.addEventListener("drop", (ev) => {
            ev.preventDefault();
            var data = JSON.parse(ev.dataTransfer.getData(textType));
            if(data.fromPlayer){
                // The amount could have changed during dragging, so we'll query current value
                // from the source inventory.
                if(sim.move_selected_inventory_item(!data.fromPlayer, idx === 0 ? "Input" : idx === 1 ? "Output" : "Burner")){
                    deselectPlayerInventory();
                    updateInventory(sim.get_player_inventory());
                    updateToolBar();
                    updateStructureInventory();
                }
            }
        }, true);
    });
    inventoryElem.style.display = 'none';

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

    let burnerItemElem = null;
    function showBurnerStatus([c, r]){
        const [burnerInventory, _] = sim.get_structure_inventory(c, r, "Burner");
        if(burnerInventory){
            burnerContainer.style.display = "block";
            const elem = inputFuelElem;
            // Clear the elements first
            // while(elem.firstChild)
            //     elem.removeChild(elem.firstChild);

            if(0 < burnerInventory.length){
                const [name, v] = burnerInventory[0];
                if(burnerItemElem === null){
                    burnerItemElem = generateItemImage(name, true, v);
                    burnerItemElem.setAttribute('draggable', 'true');
                }
                else{
                    const imageFile = getImageFile(i);
                    burnerItemElem.src = `url(${imageFile.url})`;
                    burnerItemElem.children[1].innerHTML = v;
                }
                burnerItemElem.ondragstart = function(ev){
                    console.log("dragStart");
                    // selectThisItem(this.itemName);
                    ev.dataTransfer.dropEffect = 'move';
                    // Encode information to determine item to drop into a JSON
                    ev.dataTransfer.setData(textType, JSON.stringify({
                        type: name,
                        fromPlayer: false,
                        inventoryType: "Burner",
                    }));
                };
                burnerItemElem.setAttribute('class', 'noselect');
                elem.appendChild(burnerItemElem);
            }
            else if(burnerItemElem){
                elem.removeChild(burnerItemElem);
                burnerItemElem = null;
            }

            const burnerEnergy = sim.get_structure_burner_energy(c, r, true);
            if(burnerEnergy){
                const burnerEnergyElem = document.getElementById('burnerEnergy');
                burnerEnergyElem.style.width = `${burnerEnergy[0] / burnerEnergy[1] * 80}px`;
            }
        }
        else{
            burnerContainer.style.display = "none";
        }
    }

    function showInventory(event){
        if(inventoryElem.style.display !== "none"){
            inventoryElem.style.display = "none";
            return;
        }
        // else if(tile.structure && tile.structure.inventory){
        else if(event){
            inventoryElem.style.display = "block";
            inventoryElem.classList = "inventoryWide";
            inventory2ClientElem.style.display = "block";
            playerElem.style.left = '370px';
            placeCenter(inventoryElem);
            bringToTop(inventoryElem);
            // var recipeSelectButtonElem = document.getElementById('recipeSelectButton');
            recipeSelectButtonElem.style.display = !event.recipe_enable ? "none" : "block";
            // toolTip.style.display = "none"; // Hide the tool tip for "Click to oepn inventory"
            const pos = event.pos;
            updateInventoryInt(inventoryContentElem, sim, false, sim.get_structure_inventory(pos[0], pos[1], "Input"), inputInventoryTitleElem);
            updateInventoryInt(outputInventoryContentElem, sim, false, sim.get_structure_inventory(pos[0], pos[1], "Output"), outputInventoryTitleElem);
            showBurnerStatus(pos);
        }
        else{
            inventoryElem.style.display = "block";
            inventoryElem.classList = "inventoryNarrow";
            inventory2ClientElem.style.display = "none";
            recipeSelectButtonElem.style.display = "none";
            playerElem.style.left = "40px";
        }
    }

    let recipeTarget = null;

    function recipeDraw(recipe, onclick){
        const recipeBox = document.createElement("div");
        recipeBox.className = "recipe-box";
        recipeBox.onclick = onclick;
        const timeIcon = document.createElement("span");
        timeIcon.style.display = "inline-block";
        timeIcon.style.margin = "1px";
        timeIcon.innerHTML = getHTML(generateItemImage("time", true, recipe.recipe_time), true);
        recipeBox.appendChild(timeIcon);
        const inputBox = document.createElement("span");
        inputBox.style.display = "inline-block";
        inputBox.style.width = "50%";
        for(var k in recipe.input)
            inputBox.innerHTML += getHTML(generateItemImage(k, true, recipe.input[k]), true);
        recipeBox.appendChild(inputBox);
        const arrowImg = document.createElement("img");
        arrowImg.src = rightarrow;
        arrowImg.style.width = "20px";
        arrowImg.style.height = "32px";
        recipeBox.appendChild(arrowImg);
        const outputBox = document.createElement("span");
        outputBox.style.display = "inline-block";
        outputBox.style.width = "10%";
        for(var k in recipe.output)
            outputBox.innerHTML += getHTML(generateItemImage(k, true, recipe.output[k]), true);
        recipeBox.appendChild(outputBox);
        return recipeBox;
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
            while(0 < recipeSelectorContent.childNodes.length)
                recipeSelectorContent.removeChild(recipeSelectorContent.childNodes[0]);
            for(var i = 0; i < recipes.length; i++){
                const index = i;
                recipeSelectorContent.appendChild(recipeDraw(recipes[i], (evt) => {
                    sim.select_recipe(recipeTarget[0], recipeTarget[1], index);
                    recipeSelector.style.display = "none";
                }));
            }
            // recipeSelectorContent.innerHTML = text;
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
        var elemRect = elem.getBoundingClientRect();
        var bodyRect = document.body.getBoundingClientRect();
        elem.style.left = ((bodyRect.width - elemRect.width) / 2) + 'px';
        elem.style.top = ((bodyRect.height - elemRect.height) / 2) + 'px';
    }

    placeCenter(inventoryElem);
    windowOrder.push(inventoryElem);

    const recipeSelectButtonElem = document.getElementById('recipeSelectButton');
    recipeSelectButtonElem.onclick = showRecipeSelect;

    var recipeSelectorDragStart = null;

    const recipeSelectorTitle = document.getElementById('recipeSelectorTitle');
    const recipeSelector = document.getElementById('recipeSelector');
    if(recipeSelectorTitle && recipeSelector){
        placeCenter(recipeSelector);
        windowOrder.push(recipeSelector);
        recipeSelectorTitle.addEventListener('mousedown', function(evt){
            dragWindowMouseDown(evt, recipeSelector, recipeSelectorDragStart);
        })
    }

    const playerElem = document.createElement('div');
    playerElem.style.position = 'absolute';
    playerElem.style.left = '370px';
    playerElem.style.top = '20px';
    playerElem.style.width = (320) + 'px';
    playerElem.style.height = (160) + 'px';
    inventoryElem.appendChild(playerElem);

    const playerInventoryTitleElem = document.createElement('div');
    playerInventoryTitleElem.innerHTML = "Player inventory";
    playerInventoryTitleElem.classList = "inventoryTitle";
    playerElem.appendChild(playerInventoryTitleElem);

    const playerInventoryContainerElem = document.createElement('div');
    playerInventoryContainerElem.style.overflow = 'hidden';
    playerInventoryContainerElem.style.borderStyle = 'solid';
    playerInventoryContainerElem.style.borderWidth = '1px';
    playerInventoryContainerElem.style.border = '1px solid #00f';
    playerInventoryContainerElem.style.backgroundColor = '#ffff7f';
    playerInventoryContainerElem.style.height = (160) + 'px';
    playerInventoryContainerElem.style.margin = '3px';
    playerElem.appendChild(playerInventoryContainerElem);

    const playerInventoryElem = document.createElement('div');
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
            if(sim.move_selected_inventory_item(!data.fromPlayer, data.inventoryType)){
                deselectPlayerInventory();
                updateInventory(sim.get_player_inventory());
                updateToolBar();
                updateStructureInventory();
            }
        }
    }
    playerInventoryElem.onclick = function(){onInventoryClick(true, true)};
    playerInventoryContainerElem.appendChild(playerInventoryElem);

    function onInventoryClick(isPlayer, isInput){
        // Update only if the selected inventory is the other one from destination.
        if(sim.get_selected_inventory() !== null){
            if(sim.move_selected_inventory_item(isPlayer, isInput ? "Input" : "Output")){
                deselectPlayerInventory();
                updateInventory(sim.get_player_inventory());
                updateToolBar();
                updateStructureInventory();
            }
        }
    }

    let dragging = null;
    canvas.addEventListener("mousedown", function(evt){
        processEvents(sim.mouse_down([evt.offsetX, evt.offsetY], evt.button));
        if(evt.button === 0)
            dragging = [evt.offsetX, evt.offsetY, false];
        evt.stopPropagation();
        evt.preventDefault();
        return false;
    });
    canvas.addEventListener("contextmenu", function(evt){
        evt.preventDefault();
    });
    canvas.addEventListener("mousemove", function(evt){
        if(!paused)
            sim.mouse_move([evt.offsetX, evt.offsetY]);
        if(dragging){
            sim.delta_viewport_pos(evt.offsetX - dragging[0], evt.offsetY - dragging[1], true);
            dragging = [evt.offsetX, evt.offsetY, true];
        }
    });
    canvas.addEventListener("mouseup", (evt) => {
        if(!dragging || !dragging[2]){
            if(!paused)
                processEvents(sim.mouse_up([evt.offsetX, evt.offsetY], evt.button));
        }
        dragging = null;
    })

    canvas.addEventListener("mouseleave", function(evt){
        if(!paused)
            sim.mouse_leave([evt.offsetX, evt.offsetY]);
        dragging = null;
    });

    canvas.addEventListener("wheel", function(evt){
        if(!paused){
            sim.mouse_wheel(evt.deltaY, evt.offsetX, evt.offsetY);
        }
        evt.preventDefault();
    });

    function onKeyDown(event){
        if(event.keyCode === 18){ // Alt key
            altModeBox.checked = !altModeBox.checked;
            sim.set_alt_mode(altModeBox.checked);
            event.preventDefault();
            return;
        }
        const result = sim.on_key_down(event.keyCode);
        if(result){
            if(result[0] === "ShowInventory"){
                showInventory();
            }
            updateToolBarImage();
            updateToolCursor();
            event.preventDefault();
        }
        else if(event.keyCode === 80)
            paused = !paused;
    }
    window.addEventListener( 'keydown', onKeyDown, false );

    try{
        sim.load_game();
    }
    catch(e){
        console.error(e);
    }

    updateToolBarImage();

    window.addEventListener( "beforeunload", () => {
        sim.save_game();
        localStorage.setItem("FactorishWasmViewSettings", JSON.stringify({
            "headerVisible": headerContainer.style.display !== "none",
        }));
    });

    const copyButton = document.getElementById("copyButton");
    copyButton.onclick = () => {
        const copyText = document.getElementById('saveText');
        copyText.value = sim.serialize_game();

        copyText.select();
        copyText.setSelectionRange(0, 99999); /*For mobile devices*/

        document.execCommand("copy");
    };

    const saveButton = document.getElementById("saveButton");
    saveButton.onclick = () => {
        var textFileAsBlob = new Blob([sim.serialize_game()], {
            type: 'text/json'
        });
        var fileNameToSaveAs = "save.json";
    
        var downloadLink = document.createElement("a");
        downloadLink.download = fileNameToSaveAs;
        downloadLink.innerHTML = "Download File";
        let appended = false;
        if (window.webkitURL != null) {
            downloadLink.href = window.webkitURL.createObjectURL(textFileAsBlob);
        }
        else {
            downloadLink.href = window.URL.createObjectURL(textFileAsBlob);
            downloadLink.style.display = "none";
            document.body.appendChild(downloadLink);
            appended = true;
        }
        downloadLink.click();
        if(appended) {
            document.body.removeChild(downloadLink);
        }
    };

    const body = document.body;
    body.addEventListener("mousemove", (evt) => {
        let mousePos = [evt.clientX, evt.clientY];
        mouseIcon.style.left = `${mousePos[0]}px`;
        mouseIcon.style.top = `${mousePos[1]}px`;
    });

    const loadFile = document.getElementById('loadFile');
    loadFile.addEventListener('change', (event) => {
        const reader = new FileReader();
        reader.onload = (event) => {
            sim.deserialize_game(event.target.result);
        };
        reader.readAsText(event.target.files[0]);
    });

    const loadButton = document.getElementById("loadButton");
    loadButton.onclick = () => {
        loadFile.click();
    };

    updateToolBar();

    updateInventory(sim.get_player_inventory());

    function processEvents(events){
        if(!events)
            return;
        for(let event of events){
            if(event.UpdateStructureInventory && event.UpdateStructureInventory instanceof Array){
                // console.log(`updateStructureInventory event received ${event}`);
                updateStructureInventory(event.UpdateStructureInventory);
            }
            if(event === "UpdatePlayerInventory"){
                console.log("UpdatePlayerInventory event received");
                updateInventory(sim.get_player_inventory());
                updateToolBar();
            }
            else if(event.ShowInventoryAt && event.ShowInventoryAt instanceof Object){
                showInventory(event.ShowInventoryAt);
            }
        }
    }

    const generateBoard = document.getElementById("generateBoard");
    generateBoard.addEventListener("click", () => {
        const sizeStr = document.getElementById("sizeSelect").value;
        if(sizeStr === "unlimited"){
            xsize = ysize = 128;
            unlimited = true;
        }
        else{
            xsize = ysize = parseInt(sizeStr);
            unlimited = false;
        }
        sim = new FactorishState(
            {
                width: xsize,
                height: ysize,
                unlimited,
                terrain_seed: terrainSeed,
                water_noise_threshold: waterNoiseThreshold,
                resource_amount: resourceAmount,
                noise_scale: noiseScale,
                noise_threshold: noiseThreshold,
                noise_octaves: noiseOctaves,
            },
            updateInventory,
            popupText,
            scenarioSelectElem.value,
            context,
            loadedImages);
        try{
            sim.render_init(canvas, infoElem, loadedImages);
            sim.render_gl_init(context);
        } catch(e) {
            alert(`FactorishState.render_init failed: ${e}`);
        }
    });

    const altModeBox = document.getElementById("altModeBox");
    altModeBox.addEventListener("click", () => sim.set_alt_mode(altModeBox.checked));
    const showDebugBBox = document.getElementById("showDebugBBox");
    showDebugBBox.addEventListener("click", () => sim.set_debug_bbox(showDebugBBox.checked));
    const showDebugFluidBox = document.getElementById("showDebugFluidBox");
    showDebugFluidBox.addEventListener("click", () => sim.set_debug_fluidbox(showDebugFluidBox.checked));
    const showDebugPowerNetwork = document.getElementById("showDebugPowerNetwork");
    showDebugPowerNetwork.addEventListener("click", () => sim.set_debug_power_network(showDebugPowerNetwork.checked));
    const showPerfGraph = document.getElementById("showPerfGraph");
    showPerfGraph.addEventListener("click", updatePerfVisibility);
    const useWebGLInstancing = document.getElementById("useWebGLInstancing");
    useWebGLInstancing.addEventListener("click", () => sim.set_use_webgl_instancing(useWebGLInstancing.checked));

    function updatePerfVisibility() {
        perfElem.style.display = showPerfGraph.checked ? "block" : "none";
        perfLabel.style.display = showPerfGraph.checked ? "block" : "none";
    }

    updatePerfVisibility();

    window.setInterval(function(){
        if(!paused)
            processEvents(sim.simulate(0.05));
        try{
            let result = sim.render_gl(context);
        }
        catch(e){
            console.error(e);
        }

        const selPos = sim.get_selected_inventory();
        if(selPos){
            showBurnerStatus(selPos);
        }

        const minimapData = sim.render_minimap(miniMapSize, miniMapSize);
        const viewportScale = sim.get_viewport_scale();
        if(minimapData){
            (async () => {
                const imageBitmap = await createImageBitmap(minimapData);
                miniMapContext.fillStyle = "#7f7f7f";
                miniMapContext.fillRect(0, 0, miniMapSize, miniMapSize);
                miniMapContext.drawImage(imageBitmap, 0, 0, miniMapSize, miniMapSize, 0, 0, miniMapSize, miniMapSize);
                miniMapContext.strokeStyle = "blue";
                miniMapContext.lineWidth = 1.;
                const miniMapRect = miniMapElem.getBoundingClientRect();
                const viewport = canvas.getBoundingClientRect();
                miniMapContext.strokeRect(
                    (miniMapRect.width - viewport.width / 32. / viewportScale) / 2,
                    (miniMapRect.height - viewport.height / 32. / viewportScale) / 2,
                    viewport.width / 32. / viewportScale,
                    viewport.height / 32. / viewportScale,
                );
            })()
        }

        animatePopupTexts();

        if(showPerfGraph.checked){
            const colors = ["#fff", "#ff3f3f", "#7f7fff", "#00ff00", "#ff00ff", "#fff"];
            while(perfLabel.firstChild) perfLabel.removeChild(perfLabel.firstChild);
            sim.render_perf(perfContext).forEach((text, idx) => {
                const elem = document.createElement("div");
                elem.innerHTML = text;
                elem.style.color = colors[idx % colors.length];
                perfLabel.appendChild(elem);
            });
        }
        // console.log(result);
    }, 50);
    // simulate()
})();
