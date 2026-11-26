#!/usr/bin/env python3
"""Restructure "Fuel Supply Planning Simulator.xlsx" into the Dynamist spreadsheet template.

Usage:   python3 -I restructure_fuel_workbook.py "<path to the .xlsx>" "<output folder>"

Reads the *calculated values* the workbook already holds (nothing is recalculated here) and writes
one CSV file per template sheet (a folder of CSVs imports directly in Dynamist: File -> Import
model from spreadsheet..., choose the folder; or run `dynamist --convert <folder> <file.xlsx>`).
It also writes fuel-supply-expected.csv (next to the folder): the workbook's own answers per tank, used to check the model.

What the workbook does, in the template's words (see ../Spreadsheet-Template.md):
  * 57 tanks (plant x fuel) in 4 regions, all with the same structure  -> two SETS: Region, Tank
  * each tank holds a stock (MB); trucks deliver (215 barrels each, limited by the unloading skids);
    the plant burns fuel every day (forecast)                        -> stocks and flows PER TANK
  * "Optimal plan": deliver what is burnt plus an even share of the gap to the target stock
    (Max stock x capacity) so the tank is full by the target day, never negative, whole trucks,
    at most what the skids can unload per day (Plan_Engine columns O-AH)
  * "Current plan": the trucks in the plan query, projected the same way  -> a second stock per tank
  * region and network totals                                        -> TOTAL(...)
Days are numbered by day of the month; the model starts the day after the as-of date.
"""
import csv, os, sys, zipfile
import xml.etree.ElementTree as ET

NS = {'m': 'http://schemas.openxmlformats.org/spreadsheetml/2006/main',
      'r': 'http://schemas.openxmlformats.org/officeDocument/2006/relationships'}


def load_book(path):
    z = zipfile.ZipFile(path)
    ss = [''.join(t.text or '' for t in si.iter('{%s}t' % NS['m']))
          for si in ET.fromstring(z.read('xl/sharedStrings.xml')).findall('m:si', NS)]
    wb = ET.fromstring(z.read('xl/workbook.xml'))
    rels = {r.get('Id'): r.get('Target') for r in ET.fromstring(z.read('xl/_rels/workbook.xml.rels'))}
    files = {s.get('name'): rels[s.get('{%s}id' % NS['r'])] for s in wb.find('m:sheets', NS)}

    def sheet(name):
        root = ET.fromstring(z.read('xl/' + files[name].replace('/xl/', '').lstrip('/')))
        cells = {}
        for row in root.find('m:sheetData', NS):
            for c in row:
                v = c.find('m:v', NS)
                val = v.text if v is not None else None
                if c.get('t') == 's' and val is not None:
                    val = ss[int(val)]
                elif c.get('t') in (None, 'n') and val is not None:
                    try:
                        val = float(val)
                    except ValueError:
                        pass
                cells[c.get('r')] = val
        return cells
    return sheet


def num(x):
    return x if isinstance(x, float) else None


def fmt(x):
    if x is None:
        return ''
    if isinstance(x, float):
        return str(int(x)) if x == int(x) and abs(x) < 1e12 else repr(round(x, 9))
    return str(x)


# Which part of the system each row belongs to: the model opens as an overview of these five
# modules, each drilling down into its own diagram (the "Module" column of the template).
MODULES = {
    'Inputs & limits': ['Target day', 'Barrels per truck', 'Capacity', 'Max fill', 'Min fill', 'Max trucks per day',
                        'Ready', 'Opening stock', 'Consumption forecast', 'Current plan trucks', 'Target stock'],
    'Plans / Optimal plan / Target & gap': ['Days left', 'Gap per day', 'Required MB'],
    'Plans / Optimal plan / Trucks & skids': ['Trucks needed', 'Trucks planned'],
    'Plans / Optimal plan / Tank balance': ['Stock optimal plan', 'Delivery optimal plan', 'Burn optimal plan',
                                            'Closing stock optimal', 'Closing fill optimal'],
    'Plans / Current plan': ['Stock current plan', 'Delivery current plan', 'Burn current plan', 'Closing stock current',
                             'Closing fill current', 'Shortage vs target', 'Below minimum'],
}

# A module's label is a path: "Plans / Optimal plan / Tank balance" is a module inside a module
# inside a module, so the design can be read at any level of detail.
LEVERS = ['Target day', 'Barrels per truck', 'Max fill', 'Max trucks per day']

MODULE_PURPOSE = {
    'Inputs & limits': "What each tank needs and is allowed: size, fill limits, unloading skids, today's stock, the daily burn forecast.",
    'Plans': 'The two ways of supplying the tanks, side by side.',
    'Plans / Optimal plan': 'Delivers what is burnt plus an even share of the gap, so every tank is at target on the target day.',
    'Plans / Optimal plan / Target & gap': 'How far each tank is from its target and how much must arrive today to close it.',
    'Plans / Optimal plan / Trucks & skids': 'Turns the amount needed into whole trucks, never more than the skids can unload.',
    'Plans / Optimal plan / Tank balance': 'The tank itself: what arrives, what is burnt, what is left.',
    'Plans / Current plan': 'Runs the trucks already planned forward and shows where that leaves each tank against its target and minimum.',
    'Totals': 'The same results added up, for comparison.',
    'Totals / Region totals': 'Adds the tanks up by region, to compare areas.',
    'Totals / Network totals': 'Adds everything up: stock, trucks and shortage for the whole network.',
}


def module_of(name):
    for mod, names in MODULES.items():
        if name in names:
            return mod
    if name.startswith('Region '):
        return 'Totals / Region totals'
    if name.startswith('Network '):
        return 'Totals / Network totals'
    return ''


def write(folder, name, rows):
    if name in ('Parameters', 'Stocks', 'Flows', 'Variables', 'Lookups'):
        rows = [rows[0] + ['Module']] + [r + [module_of(r[0])] for r in rows[1:]]
    if name == 'Parameters':
        # the few variables that drive the model: they get the controls (the "Lever" column)
        rows = [rows[0] + ['Lever']] + [r + ['yes' if r[0] in LEVERS else ''] for r in rows[1:]]
    with open(os.path.join(folder, name + '.csv'), 'w', newline='', encoding='utf-8') as f:
        csv.writer(f).writerows(rows)


def main(src, out):
    os.makedirs(out, exist_ok=True)
    sheet = load_book(src)
    ov, data, ps, guide = sheet('Plan_Overview'), sheet('Data'), sheet('Plan_Settings'), sheet('Guide')

    as_of = num(ps['B13'])
    target_dom = num(ps['B8'])
    bbl = num(guide['B9'])
    month_start = num(guide['B8'])
    first_plan_day = int(as_of - month_start) + 2          # day of month after the as-of date
    last_day = 30

    # ---- tanks, one row per plant and fuel on Plan_Overview
    tanks = []
    r = 8
    while ov.get(f'A{r}') is not None:
        plant, fuel = ov[f'C{r}'], ov[f'D{r}']
        tanks.append(dict(
            row=r, plant=plant, fuel=fuel, name=f'{plant} - {fuel}', region=ov[f'B{r}'],
            capacity=num(ov.get(f'F{r}')), max_fill=num(ov.get(f'G{r}')), min_fill=num(ov.get(f'H{r}')),
            opening=num(ov.get(f'I{r}')), max_trucks=num(ov.get(f'M{r}')), ready=num(ov.get(f'AH{r}')),
            opt_trucks=num(ov.get(f'X{r}')), fill_opt=num(ov.get(f'Z{r}')), fill_cur=num(ov.get(f'AA{r}')),
            shortage=num(ov.get(f'AB{r}'))))
        r += 1
    regions = []
    for t in tanks:
        if t['region'] not in regions:
            regions.append(t['region'])
    by_key = {(t['plant'], t['fuel']): t for t in tanks}

    # ---- daily series per tank from the Data sheet: consumption (MB) and the plan's trucks
    cons = {t['name']: {} for t in tanks}
    trucks = {t['name']: {} for t in tanks}
    r = 2
    while data.get(f'B{r}') is not None:
        t = by_key.get((data[f'B{r}'], data[f'C{r}']))
        d, q, e = num(data.get(f'A{r}')), num(data.get(f'Q{r}')), num(data.get(f'E{r}'))
        if t and d is not None:
            day = int(d - month_start) + 1
            if q is not None:
                cons[t['name']][day] = q
            if e is not None:
                trucks[t['name']][day] = e
        r += 1

    # ---------------------------------------------------------------- the template sheets
    write(out, 'Modules', [['Module', 'Description']] + [[k, v] for k, v in MODULE_PURPOSE.items()])

    write(out, 'Settings', [
        ['Setting', 'Value'],
        ['Title', 'Fuel supply planning, November 2026'],
        ['Initial time', first_plan_day],
        ['Final time', last_day + 1],
        ['Time step', 1],
        ['Time units', 'Day of November'],
        ['Method', 'Euler']])

    write(out, 'Sets', [['Set', 'Item', 'Belongs to']] +
          [['Region', g, ''] for g in regions] +
          [['Tank', t['name'], t['region']] for t in tanks])

    P = [['Name', 'Value', 'Units', 'Min', 'Max', 'Description', 'Per', 'Combine']]
    P += [
        ['Target day', fmt(target_dom), 'Day of month', 15, 30, 'Every tank should reach its target stock by the end of this day and be held there.', '', ''],
        ['Barrels per truck', fmt(bbl), 'Barrel/Truck', 150, 300, 'One truck carries this many barrels (1000 barrels = 1 MB).', '', ''],
        ['Capacity', 0, 'MB', 0, 1000, 'Tank capacity (average of the plan query).', 'Tank', ''],
        ['Max fill', 0.9, 'Fraction', 0.3, 1, 'Maximum filling level the power plant allows; it is also the target the optimal plan fills to (Stock_Limits).', 'Tank', ''],
        ['Min fill', 0.85, 'Fraction', 0.2, 1, 'Minimum stock level (SFI); below it a tank is at risk.', 'Tank', ''],
        ['Max trucks per day', 99999, 'Truck/Day', 0, 400, 'Most trucks the unloading skids can take in a day (Unloading_Skids). 99999 = no skid data, no limit applied.', 'Tank', ''],
        ['Ready', 1, 'Yes/No', 0, 1, '1 when capacity, max fill and stock are known; tanks that are not ready get no optimal plan.', 'Tank', ''],
        ['Opening stock', 0, 'MB', 0, 1000, 'Actual stock on the as-of date.', 'Tank', '']]
    write(out, 'Parameters', P)

    write(out, 'Stocks', [
        ['Name', 'Initial value', 'Units', 'Never negative', 'Description', 'Per', 'Combine'],
        ['Stock optimal plan', 'Opening stock', 'MB', '', 'Tank stock if the optimal plan is followed.', 'Tank', ''],
        ['Stock current plan', 'Opening stock', 'MB', '', 'Tank stock if the current plan (trucks in the plan query) is followed.', 'Tank', '']])

    write(out, 'Flows', [
        ['Name', 'Equation', 'From stock', 'To stock', 'Units', 'Description', 'Per', 'Combine'],
        ['Delivery optimal plan', 'Trucks planned * Barrels per truck / 1000', '', 'Stock optimal plan', 'MB/Day', 'Trucks of the optimal plan unloaded today.', 'Tank', ''],
        ['Burn optimal plan', 'Consumption forecast', 'Stock optimal plan', '', 'MB/Day', 'What the power plant burns today (forecast).', 'Tank', ''],
        ['Delivery current plan', 'Current plan trucks * Barrels per truck / 1000', '', 'Stock current plan', 'MB/Day', 'Trucks the current plan brings today.', 'Tank', ''],
        ['Burn current plan', 'Consumption forecast', 'Stock current plan', '', 'MB/Day', 'What the power plant burns today (forecast).', 'Tank', '']])

    write(out, 'Variables', [
        ['Name', 'Equation', 'Units', 'Description', 'Per', 'Combine'],
        ['Target stock', 'Max fill * Capacity', 'MB', 'The stock to reach by the target day.', 'Tank', ''],
        ['Days left', 'MAX(1, Target day - TIME + 1)', 'Day', 'Days until the target day, counting today (at least 1).', 'Tank', ''],
        ['Gap per day', '(Target stock - Stock optimal plan) / Days left', 'MB/Day', 'An even share of what is missing, per day left.', 'Tank', ''],
        ['Required MB', 'MAX(0, Consumption forecast + Gap per day)', 'MB/Day', 'What must arrive today: the burn plus the gap share, never below zero.', 'Tank', ''],
        ['Trucks needed', 'CEILING(Required MB * 1000 / Barrels per truck - 0.0000005)', 'Truck/Day', 'Whole trucks for the required MB.', 'Tank', ''],
        ['Trucks planned', 'Ready * MIN(Trucks needed, Max trucks per day)', 'Truck/Day', 'Trucks of the optimal plan: limited by what the skids can unload.', 'Tank', ''],
        ['Closing stock optimal', 'Stock optimal plan + Delivery optimal plan - Burn optimal plan', 'MB', 'Stock at the end of the day, optimal plan.', 'Tank', ''],
        ['Closing stock current', 'Stock current plan + Delivery current plan - Burn current plan', 'MB', 'Stock at the end of the day, current plan.', 'Tank', ''],
        ['Closing fill optimal', 'ZIDZ(Closing stock optimal, Capacity)', 'Fraction', 'Fill level at the end of the day, optimal plan.', 'Tank', 'Average'],
        ['Closing fill current', 'ZIDZ(Closing stock current, Capacity)', 'Fraction', 'Fill level at the end of the day, current plan.', 'Tank', 'Average'],
        ['Shortage vs target', 'Ready * MAX(0, Target stock - Closing stock current)', 'MB', 'How far the current plan leaves the tank below target.', 'Tank', ''],
        ['Below minimum', 'Ready * (Closing fill current < Min fill)', 'Yes/No', '1 when the current plan leaves the tank below its minimum level.', 'Tank', ''],
        ['Region stock optimal', 'TOTAL(Stock optimal plan)', 'MB', "Stock of the region's tanks, optimal plan.", 'Region', ''],
        ['Region stock current', 'TOTAL(Stock current plan)', 'MB', "Stock of the region's tanks, current plan.", 'Region', ''],
        ['Region capacity', 'TOTAL(Capacity)', 'MB', "Capacity of the region's tanks.", 'Region', ''],
        ['Region trucks optimal', 'TOTAL(Trucks planned)', 'Truck/Day', "Trucks per day the region's tanks need, optimal plan.", 'Region', ''],
        ['Region trucks current', 'TOTAL(Current plan trucks)', 'Truck/Day', "Trucks per day in the current plan for the region.", 'Region', ''],
        ['Region shortage', 'TOTAL(Shortage vs target)', 'MB', 'Shortage against target in the region, current plan.', 'Region', ''],
        ['Region fill optimal', 'ZIDZ(Region stock optimal, Region capacity)', 'Fraction', 'Region fill level, optimal plan.', 'Region', 'Average'],
        ['Region fill current', 'ZIDZ(Region stock current, Region capacity)', 'Fraction', 'Region fill level, current plan.', 'Region', 'Average'],
        ['Network stock optimal', 'TOTAL(Stock optimal plan)', 'MB', 'All tanks together, optimal plan.', '', ''],
        ['Network stock current', 'TOTAL(Stock current plan)', 'MB', 'All tanks together, current plan.', '', ''],
        ['Network capacity', 'TOTAL(Capacity)', 'MB', 'All tank capacity.', '', ''],
        ['Network trucks optimal', 'TOTAL(Trucks planned)', 'Truck/Day', 'Trucks per day, all tanks, optimal plan.', '', ''],
        ['Network trucks current', 'TOTAL(Current plan trucks)', 'Truck/Day', 'Trucks per day, all tanks, current plan.', '', ''],
        ['Network shortage', 'TOTAL(Shortage vs target)', 'MB', 'Shortage against target, all tanks, current plan.', '', ''],
        ['Network fill optimal', 'ZIDZ(Network stock optimal, Network capacity)', 'Fraction', 'Fill level of the whole network, optimal plan.', '', ''],
        ['Network fill current', 'ZIDZ(Network stock current, Network capacity)', 'Fraction', 'Fill level of the whole network, current plan.', '', '']])

    write(out, 'Lookups', [
        ['Name', 'Input', 'Points', 'Units', 'Description', 'Per', 'Combine'],
        ['Consumption forecast', 'Time', '', 'MB/Day', 'Daily burn per tank: actual up to the as-of date, forecast after (ItemSeries).', 'Tank', ''],
        ['Current plan trucks', 'Time', '', 'Truck/Day', 'Trucks per day in the plan query, per tank (ItemSeries).', 'Tank', '']])

    vals = [['Name', 'Item', 'Value']]
    for t in tanks:
        for var, key, default in (('Capacity', 'capacity', 0), ('Max fill', 'max_fill', 0), ('Min fill', 'min_fill', None),
                                  ('Opening stock', 'opening', 0), ('Ready', 'ready', 0)):
            x = t[key]
            vals.append([var, t['name'], fmt(x if x is not None else default)])
        vals.append(['Max trucks per day', t['name'], fmt(t['max_trucks'] if t['max_trucks'] is not None else 99999)])
    write(out, 'Values', [v for v in vals if v[2] != ''])

    series = [['Name', 'Item', 'Time', 'Value']]
    for t in tanks:
        for var, src_ in (('Consumption forecast', cons), ('Current plan trucks', trucks)):
            for day in range(1, last_day + 1):
                x = src_[t['name']].get(day)
                if x is not None:
                    series.append([var, t['name'], day, fmt(x)])
    write(out, 'ItemSeries', series)

    write(out, 'Links', [
        ['Cause', 'Effect', 'Polarity', 'Delay', 'Notes'],
        ['Stock optimal plan', 'Gap per day', '-', '', 'The fuller the tank, the smaller the gap to close'],
        ['Target stock', 'Gap per day', '+', '', ''],
        ['Days left', 'Gap per day', '-', '', 'More days left, a smaller daily share'],
        ['Gap per day', 'Required MB', '+', '', ''],
        ['Consumption forecast', 'Required MB', '+', '', ''],
        ['Required MB', 'Trucks needed', '+', '', ''],
        ['Trucks needed', 'Trucks planned', '+', '', ''],
        ['Max trucks per day', 'Trucks planned', '+', '', 'Skid limit'],
        ['Trucks planned', 'Delivery optimal plan', '+', '', ''],
        ['Max fill', 'Target stock', '+', '', ''],
        ['Capacity', 'Target stock', '+', '', ''],
        ['Current plan trucks', 'Delivery current plan', '+', '', ''],
        ['Target stock', 'Shortage vs target', '+', '', ''],
        ['Closing stock current', 'Shortage vs target', '-', '', ''],
        ['Min fill', 'Below minimum', '+', '', ''],
        ['Closing fill current', 'Below minimum', '-', '', ''],
        ['Skid capacity', 'Max trucks per day', '+', '', 'More unloading skids in service, more trucks per day (concept outside the model)'],
        ['Plant tank limit', 'Max fill', '+', '', 'Capacity, overflow, safety and security limits set by the plants (Stock_Limits)'],
        ['Facility readiness', 'Ready', '+', '', 'Fuel receipt declined or unloading facilities not ready (Stock_Limits)'],
        ['Expired waybills', 'Delivery current plan', '-', 'yes', 'Scheduled trucks that expire without loading'],
        ['Truck availability', 'Trucks planned', '+', '', '']])

    write(out, 'Checks', [
        ['Name', 'Variable', 'Min', 'Max', 'When'],
        ['Shortage is never negative', 'Network shortage', 0, '', 'always'],
        ['Optimal plan never fills past the limit', 'Network fill optimal', 0, 1, 'always'],
        ['Current plan never fills past the limit', 'Network fill current', 0, 1, 'always'],
        ['Shortage at the end stays under 1000 MB (example limit)', 'Network shortage', '', 1000, 'end']])

    write(out, 'Scenarios', [
        ['Scenario', 'Variable', 'Value'],
        ['Bigger trucks', 'Barrels per truck', 250],
        ['Earlier target', 'Target day', 20],
        ['Double Jazan diesel skids', 'Max trucks per day[Jazan - Diesel]', 48],
        ['Combined', 'Barrels per truck', 250],
        ['Combined', 'Target day', 20]])

    write(os.path.join(out, '..'), 'fuel-supply-expected', [['Tank', 'Ready', 'Optimal trucks to month end', 'Fill at target, optimal plan',
                             'Fill at target, current plan', 'Shortage at target MB, current plan', 'Capacity MB']] +
          [[t['name'], fmt(t['ready']), fmt(t['opt_trucks']), fmt(t['fill_opt']), fmt(t['fill_cur']), fmt(t['shortage']), fmt(t['capacity'])]
           for t in tanks])
    print(f'{len(tanks)} tanks in {len(regions)} regions; model starts on day {first_plan_day}, target day {int(target_dom)}')


if __name__ == '__main__':
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    main(sys.argv[1], sys.argv[2])
