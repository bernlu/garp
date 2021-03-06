from operator import index
from matplotlib import pyplot as plt
from matplotlib import cm
import numpy as np
import pandas as pd
import os
import re
import collections

## data dicts

# d, e -> #cell pairs
wspd_sizes = collections.defaultdict(dict)
# d, e, depth -> #cell pairs
wspd_depth_hist = collections.defaultdict(lambda: collections.defaultdict(dict))
# d, e -> #point pairs
point_pairs = collections.defaultdict(dict)
# d, e, depth -> #point pairs
point_pairs_hist = collections.defaultdict(lambda: collections.defaultdict(dict))
# d, e -> covering error %
covering_error = collections.defaultdict(dict)
# d, e, depth -> #geometric error
geom_errors = collections.defaultdict(lambda: collections.defaultdict(dict))
# (d,e) -> time
wspd_time = collections.defaultdict(lambda: pd.Timedelta(0))

if os.isatty(0):
    analysis_path = "analysis"
else:
    analysis_path = "../analysis"

img_path = analysis_path + "/img"

## load all the things
# set name
name = "germany"
# name = "stgtregbz"

# wspd analysis 
for file in [f for f in os.listdir(analysis_path) if re.match(name + r"_d\d+_e\d+.txt", f)]:
    with open(f"{analysis_path}/{file}", "r") as file:
        # print(file)
        # state trackers for reading multi line data
        reading_ppdh, reading_geomerror, reading_cellhist = [False]*3
        cellhist_idx = 0

        # read file
        for line in file.readlines():
            # read d, e
            if re.match(r"running with *", line):
                d, e = re.findall(r"\d+[^\.]|0\.\d+", line)
                d = int(d)
                e = float(e)
            
            if "duration:" in line:
                duration = re.findall(r"\d+\.\d+.*", line)[0]
                time = pd.Timedelta(duration)
                wspd_time[d, e] += time

            
            # read |wspd| = #cell pairs
            if re.match(r"wspd size: \d+", line):
                num_cell_pairs = int(*re.findall(r"\d+", line))
                wspd_sizes[d][e] = num_cell_pairs
            
            # read covering error %, #pairs
            if re.match(r"#pairs/#potential pairs: *", line):
                covering_error_percent = float(re.findall(r"\d+\.\d+%", line)[0][:-1])
                covering_error[d][e] = covering_error_percent

                num_point_pairs = int(re.findall(r" \d+/", line)[0][:-1])
                point_pairs[d][e] = num_point_pairs
                if num_point_pairs == 0: break

            # read #point pair / depth hist
            if reading_ppdh:
                if re.match(r"}", line):
                    reading_ppdh = False
                else:
                    depth, count = re.findall(r"\d+", line)
                    depth = int(depth)
                    count = int(count)
                    point_pairs_hist[d][e][depth] = count
            
            if re.match(r"#pairs per depth: {", line):
                reading_ppdh = True   

            # read geometric error hist
            if reading_geomerror:
                if re.match(r"}", line):
                    reading_geomerror = False
                else:
                    depth, count = re.findall(r"\d+", line)
                    depth = int(depth)
                    count = int(count)
                    geom_errors[d][e][depth] = count

            if re.match(r"geometric error hist: {$", line):
                reading_geomerror = True
            
            # read cell pair depth hist
            if reading_cellhist:
                if re.match(r"]", line):
                    reading_cellhist = False
                else:
                    count = int(*re.findall(r"\d+", line)) # this is sorted by depth, starting at 0 to d (d+1 values)
                    wspd_depth_hist[d][e][cellhist_idx] = count
                    cellhist_idx += 1

            if re.match(r"pair depth histogram: \[", line):
                reading_cellhist = True
            

## move data to DataFrames

# df builder for 2d data
def df_from_dict(d, index_name='d', column_name='e') -> pd.DataFrame:
    df = pd.DataFrame(d).T
    df.index.name = index_name
    df.columns.name = column_name
    df = df.reindex(df.columns.sort_values(ascending=False), axis=1)
    df = df.reindex(df.index.sort_values(), axis=0)
    return df

cell_pairs = df_from_dict(wspd_sizes)

point_pairs = df_from_dict(point_pairs)

covering_error = df_from_dict(covering_error) # / 100


# for 3d data: one dataframe for each d

def df_3d(d) -> dict[int, pd.DataFrame]:
    return {k: df_from_dict(v, index_name='e', column_name='depth').T for k,v in d.items()}

cell_size_depth_hists = df_3d(wspd_depth_hist)
point_pairs_depth_hists = df_3d(point_pairs_hist)
geom_errors = df_3d(geom_errors)

# calc geom error
# geometric error data is generated by checking 1% of cell pairs for path intersection.
# if there is an error, the weight of that cell is added to this histogram

# first change the histogram values to expected values (1% are checked -> *100 for expected value)
geom_errors = {k: geom_errors[k] * 100 for k in geom_errors.keys()}

# then calculate error based on point pairs
geom_errors_percent = {k : geom_errors[k] / point_pairs.loc[k] for k in geom_errors.keys()}

# store mean geometric error
geom_errors = {k: geom_errors_percent[k].mean() for k in geom_errors_percent.keys()}
geom_errors = pd.DataFrame(geom_errors).T.sort_index()



## draw results, save images

# wspd sizes - d=5
ax = cell_pairs.loc[5].dropna().T.plot(logy=False, logx=True, title="Gr????e der WSPD")
ax.set_xlabel("$\epsilon$")
ax.set_ylabel("$\mid WSPD\mid$")
plt.savefig(f"{img_path}/wspd_size_d5.png", bbox_inches='tight', dpi=300)

# wspd sizes - d=[8:12]
ax = cell_pairs.loc[8:12].dropna(axis=1, how='all').drop(0.45, axis=1).T.plot(logy=True, logx=False, title="Gr????e der WSPD")
ax.axhline(y=60000000, color='grey', linestyle='dashed')
ax.set_xlabel("$\epsilon$")
ax.set_ylabel("$\mid WSPD\mid$")
plt.savefig(f"{img_path}/wspd_size_d8-12.png", bbox_inches='tight', dpi=300)

# covering error
ax = covering_error.drop(0.45, axis=1).T.plot(logy=True, logx=False, title="Abdeckungsfehler")
ax.set_xlabel("$\epsilon$")
ax.set_ylabel("Abdeckungsfehler (%)")
plt.savefig(f"{img_path}/covering_error.png", bbox_inches='tight', dpi=300)

# covering error ohne 5
ax = covering_error.loc[8:12].dropna(axis=1, how='all').drop(0.45, axis=1).T.plot(logy=True, logx=False, title="Abdeckungsfehler")
ax.set_xlabel("$\epsilon$")
ax.set_ylabel("Abdeckungsfehler (%)")
plt.savefig(f"{img_path}/covering_error_8_12.png", bbox_inches='tight', dpi=300)

colors = [(0.5333333333333333, 0.9137254901960784, 0.6039215686274509),
 (0.8784313725490196, 0.16470588235294117, 0.3803921568627451),
 (0.12156862745098039, 0.6470588235294118, 0.3843137254901961),
 (0.3607843137254902, 0.2235294117647059, 0.7058823529411765),
 (0.6627450980392157, 0.9098039215686274, 0.10196078431372549),
 (0.15294117647058825, 0.4235294117647059, 0.7137254901960784),
 (0.8352941176470589, 0.5843137254901961, 0.8509803921568627),
 (0.06666666666666667, 0.36470588235294116, 0.3215686274509804),
 (0.984313725490196, 0.47058823529411764, 0.06274509803921569),
 (0.4549019607843137, 0.20784313725490197, 0.00784313725490196),
 (0.44313725490196076, 0.8274509803921568, 0.9568627450980393)]

# point pair depth hist
for k, hist in point_pairs_depth_hists.items():
    ax = (hist ).T.plot(kind='bar', #/ hist.sum()
        stacked=True, 
        logy=False, 
        title=f"Punktpaare pro Tiefe in der WSPD f??r d={k}",
        color=colors[12-k:])
    handles, labels = ax.get_legend_handles_labels()
    ax.legend(handles[::-1], labels[::-1], bbox_to_anchor=(1.0, 1.0))
    ax.set_xlabel("$\epsilon$")
    ax.set_ylabel("Anzahl der Punktpaare")
    plt.savefig(f"{img_path}/point_pair_hist_d={k}.png", bbox_inches='tight', dpi=300)

# geometric error
ax = (geom_errors * 100).T.plot(logy=False, logx=False, title="Geometrische Abweichung")
ax.set_xlabel("$\epsilon$")
ax.set_ylabel("Geometrische Abweichung (%)")
plt.savefig(f"{img_path}/geom_error.png", bbox_inches='tight', dpi=300)

ax = (geom_errors * 100).loc[8:12].dropna(axis=1, how='all').T.plot(logy=False, logx=False, title="Geometrische Abweichung")
ax.set_xlabel("$\epsilon$")
ax.set_ylabel("Geometrische Abweichung (%)")
plt.savefig(f"{img_path}/geom_error_8_12.png", bbox_inches='tight', dpi=300)



# hittingset analysis

num_paths = {}          # (d, e) -> #paths
path_weights = {}      # (d, e) -> #paths * weight
iteration_stats = collections.defaultdict(dict)   # (d, e) -> [iteration, it time, #hit paths, #paths left, weighted #hit paths]
hs_duration = {}       # (d, e) -> duration
lower_bound = {}        # (d, e) -> lower bound
hs_size = {}            # (d, e) -> hs size

for filename in [f for f in os.listdir(analysis_path) if re.match(name + r"_hs_d\d+_e\d+.txt", f)]:
    with open(f"{analysis_path}/{filename}", "r") as file:
        d = int(re.findall(r"d\d+", filename)[0][1:])
        e = re.findall(r"e\d+", filename)[0][1:]
        if e == "10":
            e = 1
        else:
            e = float("0." + e)
        
        # state trackers for reading multi line data
        reading_iter_stats = False

        # read file
        for line in file.readlines():
            if re.match(r"number of paths: \d+", line):
                num_paths[d, e] = int(*re.findall(r"\d+", line))
            
            if re.match(r"sum of path weights: \d+", line):
                path_weights[d, e] = int(*re.findall(r"\d+", line))

            if re.match(r"hitting set found. duration: \d+", line):
                hs_duration[d, e] = pd.Timedelta(*re.findall(r"\d+\.\d+[a-z]+", line))
                
            if re.match(r"lower bound: \d+", line):
                lower_bound[d, e] = int(*re.findall(r"\d+", line))

            if re.match(r"hs size: \d+", line):
                hs_size[d, e] = int(*re.findall(r"\d+", line))
            
            # read iteration stats
            if reading_iter_stats:
                if re.match(r"hitting set found.", line):
                    reading_iter_stats = False
                else:
                    i, time, hit, left, weighted_hit = re.findall(r"[^,]+", line)
                    
                    iteration_stats[d, e][int(i)] = [pd.Timedelta(time), int(hit), int(left), int(weighted_hit)]

            if re.match(r"iteration, iteration time, #hit paths, #paths left, weighted #hit paths", line):
                reading_iter_stats = True
                

iter_stats = {k: pd.DataFrame(v).T for k, v in iteration_stats.items()}
hit_paths_hist = pd.DataFrame()
for k, v in iter_stats.items():
    v.columns = ["iteration time", "hit paths", "paths left", "hit pairs"]
    v['sum hit pairs'] = v["hit pairs"].cumsum()
    num_pairs = path_weights[k[0], k[1]]
    v['relative hit pairs'] = v['sum hit pairs'] / num_pairs * 100
    hit_paths_hist[f"d={k[0]}, $\epsilon$={k[1]}"] = v['relative hit pairs']



ax = hit_paths_hist.plot(logx=True, logy=True, title="Abdeckung des Hitting Sets")
ax.set_xlabel("Gr????e des Hitting Sets")
ax.set_ylabel("Anteil der abgedeckten Pfade")
plt.savefig(f"{img_path}/hit_paths_hist.png", bbox_inches='tight', dpi=300)

print("%hitter:")
print("d & $\epsilon$ & 90\% & 95\% & 99\% & 99.9\% & 99.99\% & 99.999\% \\\\")
print("\hline")

hit_90p = (hit_paths_hist < 90).sum()
hit_95p = (hit_paths_hist < 95).sum()
hit_99p = (hit_paths_hist < 99).sum()
hit_999p = (hit_paths_hist < 99.9).sum()
hit_9999p = (hit_paths_hist < 99.99).sum()
hit_99999p = (hit_paths_hist < 99.999).sum()

for (d, e) in num_paths.keys():
    key = f"d={d}, $\epsilon$={e}"
    print(f"{d} & {e} & {hit_90p[key]} & {hit_95p[key]} & {hit_99p[key]}& {hit_999p[key]}& {hit_9999p[key]}& {hit_99999p[key]} \\\\")



print("hittingset results.")
print("d & $\epsilon$ & Pfade & Hitting Set & Lower Bound & Laufzeit & Abdeckungsfehler \\\\")
print("\hline")

for (d, e) in num_paths.keys():
    runtime_min = round((wspd_time[d,e] + hs_duration[d,e]).total_seconds() / 60)
    print(f"{d} & {e} & ${num_paths[d,e]/1e6:.0f} * 10^6$ & {hs_size[d,e]} & {lower_bound[d,e]} & {runtime_min} min & {covering_error.loc[d,e]} \\\\")