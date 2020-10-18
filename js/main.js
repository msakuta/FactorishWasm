import init, { FactorishState } from "../pkg/factorish_js.js";

window.onload = async function(){
    await init();
    let sim = new FactorishState();

    const canvas = document.getElementById('canvas');
    const canvasSize = canvas.getBoundingClientRect();
    const ctx = canvas.getContext('2d');
    const container = document.getElementById('container2');
    const containerRect = container.getBoundingClientRect();

    const infoElem = document.createElement('div');
    infoElem.style.position = 'absolute';
    infoElem.style.backgroundColor = '#ffff7f';
    infoElem.style.border = '1px solid #00f';
    container.appendChild(infoElem);

    const tilesize = 32;
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
    var toolCursorElem;
    // Tool bar
    var toolBarElem = document.createElement('div');
    toolBarElem.style.borderStyle = 'solid';
    toolBarElem.style.borderWidth = '1px';
    toolBarElem.style.borderColor = 'red';
    toolBarElem.style.position = 'relative';
    toolBarElem.margin = '3px';
    toolBarElem.style.left = '50%';
    toolBarElem.style.width = ((toolDefs.length + 1) * tilesize + 8) + 'px';
    toolBarElem.style.height = (tilesize + 8) + 'px';
    container.appendChild(toolBarElem);
    for(var i = 0; i < toolDefs.length; i++){
        var toolContainer = document.createElement('span');
        toolContainer.style.position = 'absolute';
        toolContainer.style.display = 'inline-block';
        toolContainer.style.width = '31px';
        toolContainer.style.height = '31px';
        toolContainer.style.top = '4px';
        toolContainer.style.left = (32.0 * i + 4) + 'px';
        toolContainer.style.border = '1px black solid';

        var toolElem = document.createElement("div");
        toolElems.push(toolElem);
        toolElem.style.width = '31px';
        toolElem.style.height = '31px';
        toolElem.style.position = 'absolute';
        toolElem.style.textAlign = 'center';
        toolElem.onmousedown = function(e){
            var currentTool = toolElems.indexOf(this);
            sim.select_tool(currentTool);
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
            toolCursorElem.style.display = 'block';
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
        toolElem.style.backgroundImage = 'url(' + toolDefs[i] + ')';
        toolContainer.appendChild(toolElem);
		toolBarElem.appendChild(toolContainer);
    }
	// Set the margin after contents are initialized
	toolBarElem.style.marginLeft = (-(toolBarElem.getBoundingClientRect().width + miniMapSize + tableMargin) / 2) + 'px';

    sim.render_init(canvas, infoElem);

    canvas.addEventListener("mousedown", function(evt){
        sim.mouse_down([evt.offsetX, evt.offsetY], evt.button);
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
        sim.on_key_down(event.keyCode);
    }
    window.addEventListener( 'keydown', onKeyDown, false );

    window.setInterval(function(){
        sim.simulate(0.05);
        let result = sim.render(ctx);
        // console.log(result);
    }, 50);
    // simulate()
}
