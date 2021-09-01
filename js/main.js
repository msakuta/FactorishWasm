import rotateImage from "../img/rotate.png";
import rightarrow from "../img/rightarrow.png";
import inventory from "../img/inventory.png";

import { loadImages, getImageFile } from "./images.js";
import { FactorishState } from "../pkg/index.js";

import { createApp, nextTick } from "vue";

import InventoryWindow from "./components/InventoryWindow.vue";
import RecipeSelectorWindow from "./components/RecipeSelectorWindow.vue";
import ToolTip from "./components/ToolTip.vue";

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
    const loadingContainer = document.getElementById("loadingContainer");
    loadingContainer.style.marginLeft = `${-loadingContainer.getBoundingClientRect().width / 2}px`;
    loadingContainer.style.height = `${-loadingContainer.getBoundingClientRect().height / 2}px`;

    let canvasSize = canvas.getBoundingClientRect();
    const context = canvas.getContext('webgl', { alpha: false });

    const container = document.getElementById('container2');
    const containerRect = container.getBoundingClientRect();
    const mouseIcon = document.getElementById("mouseIcon");
    const mouseIconOverlay = document.getElementById("mouseIconOverlay");

    const vueToolTipApp = createApp(ToolTip).mount('#toolTip');

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
        structureDestroyed,
        scenarioSelectElem.value,
        context,
        loadedImages,
        );

        sim.render_init(canvas, infoElem, loadedImages);
        sim.render_gl_init(context);
    } catch(e) {
        alert(`FactorishState.render_init failed: ${e}`);
    }

    loadingContainer.style.display = "none";

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
            setItemImageToElem(mouseIcon, item[0], true);
            mouseIconOverlay.innerHTML = item[1];
        }
        else
            mouseIcon.style.display = "none";
    }

    function setToolTip(elem, text, owner=""){
        var r = elem.getBoundingClientRect();
        var cr = container.getBoundingClientRect();
        vueToolTipApp.visible = true;
        vueToolTipApp.recipeDraw = false;
        vueToolTipApp.owner = owner;
        vueToolTipApp.text = text;
        vueToolTipApp.left = (r.left - cr.left);
        vueToolTipApp.bottom = window.innerHeight - r.top;
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

    function deselectInventory(){
        sim.deselect_inventory();
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
        toolElem.onmouseleave = (_e) => vueToolTipApp.visible = false;
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
    rotateButton.onmouseleave = (_e) => vueToolTipApp.visible = false;
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
    inventoryButton.onmousedown = () => {
        showInventory();
        vueApp.placeCenter();
    };
    inventoryButton.onmouseenter = (e) => setToolTip(e.target, "<b>Inventory</b><br><i>Shortcut: (E)</i>");
    inventoryButton.onmouseleave = () => vueToolTipApp.visible = false;
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
            updateVuePlayerInventory(inventory);
        }catch(e){
            console.log(e);
        }
    }

    function structureDestroyed(isSelectedStructure){
        if(isSelectedStructure){
            vueApp.inventoryVisible = false;
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
        updateVueInputInventory(sim.get_structure_inventory(...position, "Input"));
        updateVueOutputInventory(sim.get_structure_inventory(...position, "Output"));
        updateVueStorageInventory(sim.get_structure_inventory(...position, "Storage"));
    }

    function setItemImageToElem(img, i, iconSize){
        var imageFile = getImageFile(i);
        img.style.backgroundImage = `url(${imageFile.url})`;
        var size = iconSize ? 32 : objViewSize;
        img.style.width = size + 'px';
        img.style.height = size + 'px';
        img.style.backgroundSize = size * imageFile.widthFactor + 'px ' + size * imageFile.heightFactor + 'px';
        img.setAttribute('draggable', 'false');
        return img;
    }

    const inventoryClickHandler = (getItems, invtype) => (i, evt, rightClick) => {
        console.log(`onClick${invtype}: evt.ctrlKey: ${evt.ctrlKey}`);
        const itemType = sim.get_selected_item_type();
        if(evt.ctrlKey && itemType === null){
            const items = getItems();
            if(i < items.length){
                sim.select_structure_inventory(i, invtype, rightClick);
                if(sim.move_selected_inventory_item(true, invtype, true)){
                    deselectInventory();
                    updateInventory(sim.get_player_inventory());
                    updateToolBar();
                    updateStructureInventory();
                }
                else{
                    deselectInventory();
                }
            }
            else if(sim.move_all_inventory_items(true, invtype)) {
                updateInventory(sim.get_player_inventory());
                updateToolBar();
                updateStructureInventory();
            }
        }
        else if(itemType !== null && "PlayerInventory" in itemType){
            if(sim.move_selected_inventory_item(false, invtype, false)){
                deselectInventory();
                updateInventory(sim.get_player_inventory());
                updateToolBar();
                updateStructureInventory();
            }
        }
        else if(itemType === null){
            const items = getItems();
            if(i < items.length){
                sim.select_structure_inventory(i, invtype, rightClick);
                updateMouseIcon();
                // updateInventorySelection(elem);
            }
        }
        else if(sim.get_selected_inventory()){
            deselectInventory();
        }
        evt.preventDefault();
    };

    const inventoryMouseEnterHandler = (getItems, _invtype) => (i, evt) => {
        const items = getItems();
        if(i < items.length){
            setToolTip(evt.target, `${items[i].name} (${items[i].count})`, "inventory");
        }
    };

    const inventoryMouseLeaveHandler = (_getItems, _invtype) => (i, evt) => {
        vueToolTipApp.visible = false;
    };

    function playerClickHandler(i, evt, rightClick){
        console.log(`onClickPlayer evt.ctrlKey: ${evt.ctrlKey}`);
        const itemType = sim.get_selected_item_type();
        if (itemType === null) {
            const items = vueApp.playerItems.value;
            if(evt.ctrlKey){
                if(i < items.length){
                    sim.select_player_inventory(i, rightClick);
                    // The second argument doesn't matter, but needs to be something deserializable without error.
                    const res = sim.move_selected_inventory_item(false, "Burner", true);
                    if(res){
                        deselectInventory();
                        updateInventory(sim.get_player_inventory());
                        updateToolBar();
                        updateStructureInventory();
                    }
                    else{
                        deselectInventory();
                    }
                }
                else if(sim.move_all_inventory_items(false, "Burner")){
                    updateInventory(sim.get_player_inventory());
                    updateToolBar();
                    updateStructureInventory();
                }
            }
            else{
                if (i < items.length) {
                    sim.select_player_inventory(i, rightClick);
                    updateMouseIcon();
                    // updateInventorySelection(elem);
                }
            }
        } else if ("PlayerInventory" in itemType) {
          deselectInventory();
        } else {
          const invtype = sim.get_selected_inventory_type();
          if (invtype) {
            if (sim.move_selected_inventory_item(true, invtype, false)) {
              deselectInventory();
              updateInventory(sim.get_player_inventory());
              updateToolBar();
              updateStructureInventory();
            }
            deselectInventory();
          }
        }
        evt.preventDefault();
    }

    const playerMouseEnterHandler = (i, evt) => {
        const items = vueApp.playerItems.value;
        if(i < items.length){
            setToolTip(evt.target, `${items[i].name} (${items[i].count})`, "inventory");
        }
    };

    const playerMouseLeaveHandler = (_i, _evt) => {
        vueToolTipApp.visible = false;
    };

    /// An array of window elements which holds order of z indices.
    const windowOrder = [];

    const vueApplication = createApp(
        InventoryWindow,
        {
            dragWindowMouseDown,
            inventoryClickHandler,
            inventoryMouseEnterHandler,
            inventoryMouseLeaveHandler,
            playerClickHandler,
            playerMouseEnterHandler,
            playerMouseLeaveHandler,
            showRecipeSelect,
            recipeSelectMouseEnterHandler: evt => setToolTip(evt.target, "Select a recipe", "recipe"),
            recipeSelectMouseLeaveHandler: () => vueToolTipApp.visible = false,
            bringToTop: () => bringToTop(vueApp),
        }
    );

    const vueApp = vueApplication.mount('#vueApp');

    windowOrder.push(vueApp);

    vueApp.onClose = (visible) => {
        if(!visible){
            if(vueRecipeSelector.visible)
                vueRecipeSelector.visible = false;
            if(vueToolTipApp.visible && (vueToolTipApp.owner === "inventory" || vueToolTipApp.owner === "recipe"))
                vueToolTipApp.visible = false;
        }
    };

    function updateVueInputInventory(inputInventory){
        vueApp.inputItems.value = inputInventory.length !== 0 ? inputInventory[0].map(item => {
            return {
                name: item[0],
                count: item[1],
            };
        }) : [];
    }

    function updateVueOutputInventory(inventory){
        vueApp.outputItems.value = inventory.length !== 0 ? inventory[0].map(item => {
            return {
                name: item[0],
                count: item[1],
            };
        }) : [];
    }

    function updateVueStorageInventory(inventory){
        vueApp.storageItems.value = inventory.length !== 0 ? inventory[0].map(item => {
            return {
                name: item[0],
                count: item[1],
            };
        }) : [];
    }

    function updateVuePlayerInventory(inventory){
        vueApp.playerItems.value = inventory.length !== 0 ? inventory[0].map(item => {
            return {
                name: item[0],
                count: item[1],
            };
        }) : [];
    }

    function recipeClickHandler(recipes, i, evt){
        console.log(`onClick: evt.ctrlKey: ${evt.ctrlKey}`);
        const pos = sim.get_selected_inventory();
        if(sim.select_recipe(...pos, i)){
            updateVueInputInventory(sim.get_structure_inventory(...pos, "Input"));
            vueRecipeSelector.visible = false;
            if(vueToolTipApp.visible && vueToolTipApp.owner === "recipe")
                vueToolTipApp.visible = false;
        }
        evt.preventDefault();
    };

    const recipeMouseEnterHandler = (recipes, i, evt) => {
        if(i < recipes.length){
            const elem = evt.target;
            const r = elem.getBoundingClientRect();
            const cr = container.getBoundingClientRect();
            vueToolTipApp.visible = true;
            vueToolTipApp.recipeDraw = true;
            vueToolTipApp.recipe = recipes[i];
            vueToolTipApp.left = (r.left - cr.left);
            vueToolTipApp.bottom = window.innerHeight - r.top;
        }
    };

    const vueRecipeSelector = createApp(
        RecipeSelectorWindow,
        {
            dragWindowMouseDown,
            recipeClickHandler,
            recipeMouseEnterHandler,
            recipeMouseLeaveHandler: () => vueToolTipApp.visible = false,
            bringToTop: () => bringToTop(vueRecipeSelector),
        }
    ).mount('#recipeSelector');

    windowOrder.push(vueRecipeSelector);

    function dragWindowMouseDown(evt, elem, vueApp, pos, updatePos){
        pos = [evt.screenX, evt.screenY];
        bringToTop(vueApp);
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
            updatePos(left + rel[0], top + rel[1]);
        }
        
        mousecaptorElem.addEventListener('mousemove', mousemove);
        mousecaptorElem.addEventListener('mouseup', function(evt){
            // Stop dragging a window
            elem = null;
            this.removeEventListener('mousemove', mousemove);
            this.style.display = 'none';
        });
        mousecaptorElem.addEventListener('mouseleave', function(){
            // Stop dragging a window
            elem = null;
            this.removeEventListener('mousemove', mousemove);
            this.style.display = 'none';
        })
    }

    /// Bring a window to the top on the other windows.
    function bringToTop(vueApp){
        var oldIdx = windowOrder.indexOf(vueApp);
        if(0 <= oldIdx && oldIdx < windowOrder.length - 1){
            windowOrder.splice(oldIdx, 1);
            windowOrder.push(vueApp);
            for(var i = 0; i < windowOrder.length; i++)
                windowOrder[i].zIndex = i + windowZIndex;
        }
        else{
            vueApp.zIndex = oldIdx + windowZIndex;
        }
        var mousecaptorElem = document.getElementById('mousecaptor');
        mousecaptorElem.style.zIndex = windowOrder.length + windowZIndex; // The mouse capture element comes on top of all other windows
    }

    function showBurnerStatus([c, r]){
        const [burnerInventory, _] = sim.get_structure_inventory(c, r, "Burner");
        if(burnerInventory){
            vueApp.hasBurner = true;
            vueApp.burnerItems = burnerInventory.map(item => {
                return {
                    name: item[0],
                    count: item[1],
                };
            });

            const burnerEnergy = sim.get_structure_burner_energy(c, r, true);
            // if(burnerEnergy)
            //     vueApp.burnerEnergy = burnerEnergy[0] / burnerEnergy[1] * 80;
            if(burnerEnergy){
                vueApp.burnerEnergy = burnerEnergy[0] / burnerEnergy[1];
            }
        }
        else{
            vueApp.hasBurner = false;
        }
    }

    function updateStructureProgress(pos){
        const progress = sim.get_structure_progress(...pos);
        vueApp.progress = progress || 0;
    }

    function showInventory(event){
        vueApp.inventoryVisible = !vueApp.inventoryVisible;
        if(!vueApp.inventoryVisible){
            if(vueRecipeSelector.visible)
                vueRecipeSelector.visible = false;
            if(vueToolTipApp.visible && (vueToolTipApp.owner === "inventory" || vueToolTipApp.owner === "recipe"))
                vueToolTipApp.visible = false;
            return;
        }
        else if(event){
            vueApp.hasPosition = true;
            vueApp.placeCenter();
            nextTick(() => bringToTop(vueApp));
            const pos = event.pos;

            const inputInventory = sim.get_structure_inventory(pos[0], pos[1], "Input");
            if(inputInventory && inputInventory.length !== 0){
                vueApp.hasInput = true;
                updateVueInputInventory(inputInventory);
            }
            else{
                vueApp.hasInput = false;
            }
            const outputInventory = sim.get_structure_inventory(pos[0], pos[1], "Output");
            if(outputInventory && outputInventory.length !== 0){
                vueApp.hasOutput = true;
                updateVueOutputInventory(outputInventory);
            }
            else{
                vueApp.hasOutput = false;
            }
            const storageInventory = sim.get_structure_inventory(pos[0], pos[1], "Storage");
            if(storageInventory && storageInventory.length !== 0){
                vueApp.hasStorage = true;
                updateVueStorageInventory(storageInventory);
            }
            else{
                vueApp.hasStorage = false;
            }
            showBurnerStatus(pos);
        }
        else{
            vueApp.hasPosition = false;
        }
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

    function showRecipeSelect(evt){
        evt.stopPropagation();
        if(vueRecipeSelector.visible){
            vueRecipeSelector.visible = false;
            return;
        }
        const sel_pos = sim.get_selected_inventory();
        if(sel_pos){
            vueRecipeSelector.visible = true;
            vueRecipeSelector.placeCenter();
            bringToTop(vueRecipeSelector);
            var recipes = sim.get_structure_recipes(...sel_pos);
            vueRecipeSelector.recipes = recipes;
        }
    }

    // Place a window element at the center of the container, assumes the windows have margin set in the middle.
    function placeCenter(elem){
        var elemRect = elem.getBoundingClientRect();
        var bodyRect = document.body.getBoundingClientRect();
        elem.style.left = ((bodyRect.width - elemRect.width) / 2) + 'px';
        elem.style.top = ((bodyRect.height - elemRect.height) / 2) + 'px';
    }

    var recipeSelectorDragStart = null;

    const recipeSelectorTitle = document.getElementById('recipeSelectorTitle');
    const recipeSelector = document.getElementById('recipeSelector');
    if(recipeSelectorTitle && recipeSelector){
        placeCenter(recipeSelector);
        recipeSelectorTitle.addEventListener('mousedown', function(evt){
            dragWindowMouseDown(evt, recipeSelector, recipeSelectorDragStart, (x, y) => {
                recipeSelector.style.left = x + 'px';
                recipeSelector.style.top = y + 'px';
            });
        })
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
                vueApp.placeCenter();
                if(vueApp.inventoryVisible){
                    bringToTop(vueApp);
                }
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
            updateInventory(sim.get_player_inventory());
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
            structureDestroyed,
            scenarioSelectElem.value,
            context,
            loadedImages);
        try{
            sim.render_init(canvas, infoElem, loadedImages);
            sim.render_gl_init(context);
        } catch(e) {
            alert(`FactorishState.render_init failed: ${e}`);
        }
        updateInventory(sim.get_player_inventory());
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

    container.style.display = "block";

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
            updateStructureProgress(selPos);
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
