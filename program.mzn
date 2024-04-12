% 50 raw-oil -(20)-> 5 sulfuric-heavy-fuel;
% 50 raw-oil -(20)-> 25 sulfuric-light-fuel;
% 
% 1000 sulfuric-heavy-fuel + 250 H2 -(20)-> 1000 heavy-fuel + 125 H2S;
% 3000 sulfuric-light-fuel + 500 H2 -(40)-> 3000 light-fuel + 250 H2S;
% 
% 5000 light-fuel + 1000 heavy-fuel -(16)-> 6000 diesel;
% 
% target diesel {
%     using raw-oil, H2;
%     create 1 diesel;
% }

% variables
var 0..: machine_50raw_oil_into_5sulfuric_heavy_fuel;
var 0..: machine_50raw_oil_into_25sulfuric_light_fuel;
var 0..: machine_1000sulfuric_heavy_fuel_250H2_into_1000heavy_fuel_125H2S;
var 0..: machine_500H2_3000sulfuric_light_fuel_into_3000light_fuel_250H2S;
var 0..: machine_1000heavy_fuel_5000light_fuel_into_6000diesel;

% target constraints
constraint (0+5 * 6000 * machine_1000heavy_fuel_5000light_fuel_into_6000diesel) - (0) >= 1 * 80;

% balance constraints
constraint 0+4 * 25 * machine_50raw_oil_into_25sulfuric_light_fuel >= 0+2 * 3000 * machine_500H2_3000sulfuric_light_fuel_into_3000light_fuel_250H2S;
constraint 0+2 * 3000 * machine_500H2_3000sulfuric_light_fuel_into_3000light_fuel_250H2S >= 0+5 * 5000 * machine_1000heavy_fuel_5000light_fuel_into_6000diesel;
constraint 0+4 * 5 * machine_50raw_oil_into_5sulfuric_heavy_fuel >= 0+4 * 1000 * machine_1000sulfuric_heavy_fuel_250H2_into_1000heavy_fuel_125H2S;
constraint 0+4 * 1000 * machine_1000sulfuric_heavy_fuel_250H2_into_1000heavy_fuel_125H2S >= 0+5 * 1000 * machine_1000heavy_fuel_5000light_fuel_into_6000diesel;
constraint 0+5 * 6000 * machine_1000heavy_fuel_5000light_fuel_into_6000diesel >= 0;
constraint 0+4 * 125 * machine_1000sulfuric_heavy_fuel_250H2_into_1000heavy_fuel_125H2S+2 * 250 * machine_500H2_3000sulfuric_light_fuel_into_3000light_fuel_250H2S >= 0;

solve minimize machine_50raw_oil_into_5sulfuric_heavy_fuel+machine_50raw_oil_into_25sulfuric_light_fuel+machine_1000sulfuric_heavy_fuel_250H2_into_1000heavy_fuel_125H2S+machine_500H2_3000sulfuric_light_fuel_into_3000light_fuel_250H2S+machine_1000heavy_fuel_5000light_fuel_into_6000diesel;
