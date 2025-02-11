import rotateImage from "../img/rotate.png";
import inventory from "../img/inventory.png";
import sciencePack1 from "../img/science-pack-1.png";
import menuIcon from "../img/menuIcon.png";

import { loadImages, getImageFile } from "./images.js";
import { FactorishState } from "../pkg";

import { createApp, nextTick } from "vue";

import MainMenuWindow from "./components/MainMenuWindow.vue";
import NewGameWindow from "./components/NewGameWindow.vue";
import ViewSettingsWindow from "./components/ViewSettingsWindow.vue";
import InventoryWindow from "./components/InventoryWindow.vue";
import RecipeSelectorWindow from "./components/RecipeSelectorWindow.vue";
import ResearchSelectorWindow from "./components/ResearchSelectorWindow.vue";
import ToolTip, { HTMLDraw, RecipeDraw, ResearchDraw } from "./components/ToolTip.vue";

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

    const defaultParams = {
        terrainSeed: 8913095,
        waterNoiseThreshold: 0.28,
        resourceAmount: 1000.,
        resourceDistanceFactor: 0.1,
        noiseScale: 5.,
        noiseThreshold: 0.30,
        noiseOctaves: 3,
    };

    function initPane(buttonId, containerId){
        const button = document.getElementById(buttonId);
        const container = document.getElementById(containerId);
        if(button){
            button.addEventListener("click", (event) => {
                container.style.display = container.style.display === "none" ? "block" : "none";
            });
        }
    }
    initPane("viewButton", "viewContainer");

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
    infoElem.className = 'noselect windowFrame';
    infoElem.style.position = 'absolute';
    infoElem.style.padding = '4px';
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
            terrain_seed: defaultParams.terrainSeed,
            water_noise_threshold: defaultParams.waterNoiseThreshold,
            resource_amount: defaultParams.resourceAmount,
            resource_distance_factor: defaultParams.resourceDistanceFactor,
            noise_scale: defaultParams.noiseScale,
            noise_threshold: defaultParams.noiseThreshold,
            noise_octaves: defaultParams.noiseOctaves,
        },
        updateInventory,
        popupText,
        structureDestroyed,
        "default",
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

    const mainMenuTip = document.createElement("div");
    mainMenuTip.className = "mainMenuTip";
    mainMenuTip.innerHTML = "Main Menu";
    document.body.appendChild(mainMenuTip);

    const mainMenuIcon = document.createElement("img");
    mainMenuIcon.src = menuIcon;
    mainMenuIcon.style.position = "absolute";
    mainMenuIcon.addEventListener("click", () => {
        vueMainMenuWindow.visible = !vueMainMenuWindow.visible;
        if(vueMainMenuWindow.visible)
            vueMainMenuWindow.placeCenter();
    });
    mainMenuIcon.addEventListener("mouseenter", () => {
        mainMenuTip.style.display = "block";
    });
    mainMenuIcon.addEventListener("mouseleave", () => {
        mainMenuTip.style.display = "none";
    })
    document.body.appendChild(mainMenuIcon);

    let miniMapDrag = null;
    const tilesize = 32;
    var windowZIndex = 1000;
    const objViewSize = tilesize / 2; // View size is slightly greater than hit detection radius
    const tableMargin = 10.;
    const miniMapSize = 200;
    const researchHeight = 35;

    const researchElem = document.createElement('div');
    researchElem.classList = 'noselect windowFrame';
    researchElem.style.position = 'absolute';
    researchElem.style.top = '8px';
    researchElem.style.right = '8px';
    researchElem.style.width = `${miniMapSize}px`;
    researchElem.style.height = `${researchHeight}px`;
    researchElem.style.padding = '4px';
    researchElem.onclick = evt => showReserachSelect(evt);
    container.appendChild(researchElem);

    const researchTitleElem = document.createElement('div');
    researchTitleElem.innerHTML = "Research: ";
    researchElem.appendChild(researchTitleElem);

    const researchProgressBgElem = document.createElement('div');
    researchProgressBgElem.className = "progressBarBack";
    researchProgressBgElem.style.width = `${miniMapSize - tableMargin * 2}px`;
    researchElem.appendChild(researchProgressBgElem);
    const researchProgressElem = document.createElement('div');
    researchProgressElem.className = "progressBar";
    researchProgressBgElem.appendChild(researchProgressElem);

    const miniMapElem = document.createElement('canvas');
    miniMapElem.classList = 'noselect windowFrame';
    miniMapElem.style.position = 'absolute';
    miniMapElem.style.padding = '4px';
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
    miniMapElem.style.top = '68px';
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
        vueToolTipApp.drawMode = HTMLDraw;
        vueToolTipApp.owner = owner;
        vueToolTipApp.text = text;
        vueToolTipApp.left = (r.left - cr.left);
        vueToolTipApp.top = undefined;
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
    const toolBarElem = document.getElementById('toolBar');
    toolBarElem.style.borderStyle = 'solid';
    toolBarElem.style.borderWidth = '1px';
    toolBarElem.style.borderColor = 'red';
    toolBarElem.style.position = 'absolute';
    toolBarElem.margin = '3px';
    // toolBarElem.style.top = '480px';
    // toolBarElem.style.left = '50%';
    toolBarElem.style.width = ((toolBeltSize + 3) * tilesize + 8) + 'px';
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
    inventoryButton.style.left = (32.0 * i++ + 4) + 'px';
    inventoryButton.style.border = '1px blue solid';
    inventoryButton.style.backgroundImage = `url(${inventory})`;
    inventoryButton.onmousedown = () => showInventory();
    inventoryButton.onmouseenter = (e) => setToolTip(e.target, "<b>Inventory</b><br><i>Shortcut: (E)</i>");
    inventoryButton.onmouseleave = () => vueToolTipApp.visible = false;
    toolBarElem.appendChild(inventoryButton);

    const researchButton = document.createElement('div');
    researchButton.style.width = '31px';
    researchButton.style.height = '31px';
    researchButton.style.position = 'absolute';
    researchButton.style.top = '4px';
    researchButton.style.left = (32.0 * i + 4) + 'px';
    researchButton.style.border = '1px blue solid';
    researchButton.style.backgroundImage = `url(${sciencePack1})`;
    researchButton.onmousedown = (evt) => showReserachSelect(evt);
    researchButton.onmouseenter = (e) => setToolTip(e.target, "<b>Research</b><br><i>Shortcut: (T)</i>");
    researchButton.onmouseleave = () => vueToolTipApp.visible = false;
    toolBarElem.appendChild(researchButton);

    function updateToolBarImage(){
        for(var i = 0; i < toolBarCanvases.length; i++){
            var canvasElem = toolBarCanvases[i];
            var context = canvasElem.getContext('2d');
            sim.render_tool(i, context);
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

    const mainMenuViewSettings = JSON.parse(localStorage.getItem("FactorishWasmViewSettings"));

    const vueMainMenuWindowApp = createApp(
        MainMenuWindow,
        {
            dragWindowMouseDown,
            onShowNewGame: () => {
                vueMainMenuWindow.visible = false;
                vueNewGameWindow.visible = true;
                vueNewGameWindow.placeCenter();
                bringToTop(vueNewGameWindow);
            },
            onShowViewSettings: () => {
                vueMainMenuWindow.visible = false;
                vueViewSettingsWindow.visible = true;
                vueViewSettingsWindow.placeCenter();
                bringToTop(vueViewSettingsWindow);
            },
            serializer() { return sim.serialize_game(); },
            deserializer(data) {
                sim.deserialize_game(data);
                updateInventory(sim.get_player_inventory());
            },
            bringToTop: () => bringToTop(vueMainMenuWindow),
        }
    );

    const vueMainMenuWindow = vueMainMenuWindowApp.mount('#vueMainMenuWindow');
    // Default true
    if(!mainMenuViewSettings || mainMenuViewSettings.mainMenuVisible){
        vueMainMenuWindow.visible = true;
        vueMainMenuWindow.placeCenter();
    }

    windowOrder.push(vueMainMenuWindow);

    const vueNewGameWindowApp = createApp(
        NewGameWindow,
        {
            defaultParams,
            dragWindowMouseDown,
            onNewGame: newGame,
            bringToTop: () => bringToTop(vueNewGameWindow),
        }
    );

    const vueNewGameWindow = vueNewGameWindowApp.mount('#vueNewGameWindow');

    windowOrder.push(vueNewGameWindow);


    const vueViewSettingsApp = createApp(
        ViewSettingsWindow,
        {
            onAltMode(value) { sim.set_alt_mode(value) },
            onShowDebugBBox(value) { sim.set_debug_bbox(value) },
            onShowDebugFluidBox(value) { sim.set_debug_fluidbox(value) },
            onShowDebugPowerNetwork(value) { sim.set_debug_power_network(value) },
            onShowPerfGraph() { updatePerfVisibility() },
            onUseWebGLInstancing(value) { sim.set_use_webgl_instancing(value) },
            dragWindowMouseDown,
            bringToTop: () => bringToTop(vueViewSettingsWindow),
        }
    );

    const vueViewSettingsWindow = vueViewSettingsApp.mount('#vueViewSettingsWindow');

    windowOrder.push(vueViewSettingsWindow);

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
        if(i < recipes.length && sim.select_recipe(...pos, recipes[i].index)){
            updateVueInputInventory(sim.get_structure_inventory(...pos, "Input"));
            updateVueOutputInventory(sim.get_structure_inventory(...pos, "Output"));
            updateInventory(sim.get_player_inventory());
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
            vueToolTipApp.drawMode = RecipeDraw;
            vueToolTipApp.recipe = recipes[i];
            vueToolTipApp.left = (r.left - cr.left);
            vueToolTipApp.top = undefined;
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

    function researchClickHandler(_technologies, i, evt){
        console.log(`researchClickHandler: evt.ctrlKey: ${evt.ctrlKey}`);
        if(sim.select_research(i)){
            updateResearch();
        }
        evt.preventDefault();
    };

    const researchMouseEnterHandler = (technologies, i, evt) => {
        if(i < technologies.length){
            const elem = evt.target;
            const r = elem.getBoundingClientRect();
            const cr = container.getBoundingClientRect();
            vueToolTipApp.visible = true;
            vueToolTipApp.owner = "research";
            vueToolTipApp.drawMode = ResearchDraw;
            vueToolTipApp.technology = technologies[i];
            vueToolTipApp.left = (r.left - cr.left);
            vueToolTipApp.top = r.bottom;
            vueToolTipApp.bottom = undefined;
        }
    };

    const vueResearchSelector = createApp(
        ResearchSelectorWindow,
        {
            dragWindowMouseDown,
            researchClickHandler,
            researchMouseEnterHandler,
            researchMouseLeaveHandler: () => vueToolTipApp.visible = false,
            bringToTop: () => bringToTop(vueResearchSelector),
        }
    ).mount('#researchSelector');

    windowOrder.push(vueResearchSelector);

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
        if(vueResearchSelector.visible){
            if(vueToolTipApp.visible && vueToolTipApp.owner === "research")
                vueToolTipApp.visible = false;
            vueResearchSelector.visible = false;
            return;
        }
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
        bringToTop(vueApp);
        vueApp.placeCenter();
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

    function showReserachSelect(evt){
        evt.stopPropagation();
        if(vueApp.inventoryVisible){
            vueApp.inventoryVisible = false;
            if(vueToolTipApp.visible && (vueToolTipApp.owner === "inventory" || vueToolTipApp.owner === "recipe"))
                vueToolTipApp.visible = false;
            vueRecipeSelector.visible = false;
            sim.close_structure_inventory();
        }
        if(vueResearchSelector.visible){
            if(vueToolTipApp.visible && vueToolTipApp.owner === "research")
                vueToolTipApp.visible = false;
            vueResearchSelector.visible = false;
            return;
        }
        vueResearchSelector.visible = true;
        vueResearchSelector.placeCenter();
        bringToTop(vueResearchSelector);
        vueResearchSelector.research = sim.get_research();
        const tech = sim.get_technologies();
        vueResearchSelector.technologies = tech;
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
            if(!paused){
                processEvents(sim.mouse_up([evt.offsetX, evt.offsetY], evt.button, evt.ctrlKey));
            }
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
        switch(event.keyCode){
        case 18: // Alt key
            vueViewSettingsWindow.altMode = !vueViewSettingsWindow.altMode;
            event.preventDefault();
            return;
        case 69:
            //'e'
            showInventory();
            return;
        case 84:
            // 't'
            showReserachSelect(event);
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
        // If succeeded to load a game, update the research panel which is rarely updated.
        updateResearch();
    }
    catch(e){
        console.error(e);
    }

    updateToolBarImage();

    window.addEventListener( "beforeunload", () => {
        sim.save_game();
        localStorage.setItem("FactorishWasmViewSettings", JSON.stringify({
            "mainMenuVisible": vueMainMenuWindow.visible,
        }));
    });

    const body = document.body;
    body.addEventListener("mousemove", (evt) => {
        let mousePos = [evt.clientX, evt.clientY];
        mouseIcon.style.left = `${mousePos[0]}px`;
        mouseIcon.style.top = `${mousePos[1]}px`;
    });

    updateToolBar();

    updateInventory(sim.get_player_inventory());

    function updateResearch(){
        const research = sim.get_research();
        if(vueResearchSelector.visible) {
            vueResearchSelector.research = research;
            vueResearchSelector.technologies = sim.get_technologies();
        }
        if(research){
            researchTitleElem.innerHTML = `${research.technology} (${(research.progress * 100).toFixed(0)}%)`;
            researchProgressElem.style.width = `${research.progress * 100}%`;
        }
        else{
            researchTitleElem.innerHTML = "No research";
            researchProgressElem.style.width = "0";
        }
    }

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
            else if(event === "UpdateResearch") {
                updateResearch();
            }
        }
    }

    function newGame(params){
        if(params.sizeStr === "unlimited"){
            xsize = ysize = 128;
            unlimited = true;
        }
        else{
            xsize = ysize = parseInt(params.sizeStr);
            unlimited = false;
        }
        sim = new FactorishState(
            {
                width: xsize,
                height: ysize,
                unlimited,
                terrain_seed: params.terrainSeed,
                water_noise_threshold: params.waterNoiseThreshold,
                resource_amount: params.resourceAmount,
                resource_distance_factor: params.resourceDistanceFactor,
                noise_scale: params.noiseScale,
                noise_threshold: params.noiseThreshold,
                noise_octaves: params.noiseOctaves,
            },
            updateInventory,
            popupText,
            structureDestroyed,
            params.scenario,
            context,
            loadedImages);
        try{
            sim.render_init(canvas, infoElem, loadedImages);
            sim.render_gl_init(context);
        } catch(e) {
            alert(`FactorishState.render_init failed: ${e}`);
        }
        updateInventory(sim.get_player_inventory());
    }

    function updatePerfVisibility() {
        perfElem.style.display = vueViewSettingsWindow.showPerfGraph ? "block" : "none";
        perfLabel.style.display = vueViewSettingsWindow.showPerfGraph ? "block" : "none";
    }

    container.style.display = "block";

    updatePerfVisibility();

    let globalTime = performance.now();

    function animate(){
        // The performance counter may not be very accurate (could have 1ms error), but it won't accumulate over time
        const nextTime = performance.now();
        const deltaTime = nextTime - globalTime;
        if(!paused){
            // Supposed to be 60FPS, but truth is we don't know. requestAnimationFrame would schedule the rendering
            // as fast as the device can.
            processEvents(sim.simulate(deltaTime / 1000.));
        }
        // let result = sim.render(ctx);
        let result = sim.render_gl(context);

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

        if(vueViewSettingsWindow.showPerfGraph){
            const colors = ["#fff", "#ff3f3f", "#7f7fff", "#00ff00", "#ff00ff", "#fff"];
            while(perfLabel.firstChild) perfLabel.removeChild(perfLabel.firstChild);
            sim.render_perf(perfContext).forEach((text, idx) => {
                const elem = document.createElement("div");
                elem.innerHTML = text;
                elem.style.color = colors[idx % colors.length];
                perfLabel.appendChild(elem);
            });
        }

        globalTime = nextTime;

        // console.log(result);
        requestAnimationFrame(animate);
    }

    requestAnimationFrame(animate);
    // simulate()
})();
