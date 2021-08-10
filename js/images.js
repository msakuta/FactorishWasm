import time from "../img/time.png";
import dirt from "../img/dirt.png";
import backTiles from "../img/back32.png";
import weeds from "../img/weeds.png";
import iron from "../img/iron.png";
import coal from "../img/coal.png";
import copper from "../img/copper.png";
import stone from "../img/stone.png";
import transport from "../img/transport.png";
import undergroundBelt from "../img/underbelt.png";
import splitter from "../img/splitter.png";
import chest from "../img/chest.png";
import mine from "../img/mine.png";
import assembler from "../img/assembler.png";
import furnace from "../img/furnace.png";
import waterWell from "../img/waterwell.png";
import offshorePump from "../img/offshore-pump.png";
import boiler from "../img/boiler.png";
import pipe from "../img/pipe.png";
import inserter from "../img/inserter-base.png";
import direction from "../img/direction.png";
import ore from "../img/ore.png";
import coalOre from "../img/coal-ore.png";
import copperOre from "../img/copper-ore.png";
import stoneOre from "../img/stone-ore.png";
import ironPlate from "../img/metal.png";
import steelPlate from "../img/steel-plate.png";
import copperPlate from "../img/copper-plate.png";
import copperWire from "../img/copper-wire.png";
import circuit from "../img/circuit.png";
import gear from "../img/gear.png";
import pipeItem from "../img/pipe-item.png";
import steamEngine from "../img/steam-engine.png";
import electPole from "../img/elect-pole.png";
import smoke from "../img/smoke.png";
import undergroundBeltItem from "../img/underground-belt-item.png";
import fuelAlarm from '../img/fuel-alarm.png';
import electricityAlarm from '../img/electricity-alarm.png';

// We could fetch and await in Rust code, but it's far easier to do in JavaScript runtime.
// We initiate promises at the very beginning of the initialization, and by the time we initialize everything
// we should have bitmaps ready.
export let loadImages = [
    ["dirt", dirt],
    ["backTiles", backTiles],
    ["weeds", weeds],
    ["iron", iron],
    ["coal", coal],
    ["copper", copper],
    ["stone", stone],
    ["transport", transport],
    ["undergroundBelt", undergroundBelt],
    ["chest", chest],
    ["mine", mine],
    ["furnace", furnace],
    ["assembler", assembler],
    ["boiler", boiler],
    ["steamEngine", steamEngine],
    ["electPole", electPole],
    ["splitter", splitter],
    ["waterWell", waterWell],
    ["offshorePump", offshorePump],
    ["pipe", pipe],
    ["inserter", inserter],
    ["direction", direction],
    ["ore", ore],
    ["coalOre", coalOre],
    ["ironPlate", ironPlate],
    ["copperOre", copperOre],
    ["stoneOre", stoneOre],
    ["copperPlate", copperPlate],
    ["gear", gear],
    ["copperWire", copperWire],
    ["circuit", circuit],
    ["undergroundBeltItem", undergroundBeltItem],
    ["time", time],
    ["smoke", smoke],
    ["fuelAlarm", fuelAlarm],
    ["electricityAlarm", electricityAlarm],
].map(async ([name, src]) => {
    const res = await fetch(src);
    return [name, src, await createImageBitmap(await res.blob())];
});


function getImageFileInt(type){
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
    case 'Stone Ore':
        return stoneOre;
    case 'Gear':
        return gear;
    case 'Copper Wire':
        return copperWire;
    case 'Circuit':
        return circuit;
    case 'Transport Belt':
        return transport;
    case 'Underground Belt':
        return undergroundBeltItem;
    case 'Splitter':
        return splitter;
    case 'Inserter':
        return [inserter, 2];
    case 'Chest':
        return chest;
    case 'Ore Mine':
        return [mine, 3];
    case 'Furnace':
        return [furnace, 3];
    case 'Assembler':
        return [assembler, 4];
    case 'Water Well':
        return waterWell;
    case 'Offshore Pump':
        return offshorePump;
    case 'Boiler':
        return [boiler, 3];
    case 'Pipe':
        return pipeItem;
    case 'Steam Engine':
        return [steamEngine, 3];
    case 'Electric Pole':
        return electPole;
    default:
        return "";
    }
}

class Image {
    constructor() {
        this.url = "";
        this.widthFactor = 1;
        this.heightFactor = 1;
    }
}

export function getImageFile(type){
    const image = getImageFileInt(type);
    const ret = new Image();
    if(image instanceof Array){
        ret.url = image[0];
        ret.widthFactor = image[1];
        if(2 < image.length)
            ret.heightFactor = image[2];
    }
    else
        ret.url = image;
    return ret;
}
