#!/bin/python3

import numpy as np
import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import os
import sys

from config import *


def get_category_by_index(l: list[int], index: int) -> list[str]:
    cat = []
    temp = l.copy()
    while len(temp) != 0:
        first = temp[0]
        c = first.split("_")[index]
        cat.append(c)

        temp = list(filter(lambda x: c not in x, temp))
    return cat



def plot_thing(data: list[str], title: str = "Custom metadata"):
    data.sort(key=lambda x: int(x.split("_")[2]))
    #data.sort(key=lambda x: x)
    print(data)
    flag = True

    ticks = []
    for d in data:

        label = d.split("_")[0]
        lab = d.split("_")[2]

        y = [(float(x[:-1])/1000) for x in open(f"{folder}/{d}").readlines()]
        x = [i + 1 for i in range(len(y))]
        label = label.replace("normal ", "")
        label += "M"
        foo,  = plt.plot(x, y, label=label, linewidth=WIDTH)

        #label = label.replace("normal", "")
        #label = label.replace("cache", "cache ")
        #label += "10"
        #label += "M"

        ticks = range(1, len(x)+1)
    flag = False



    ax.xaxis.set_major_locator(ticker.MaxNLocator(integer=True))
    ax.tick_params(labelsize=FONTSIZE)
    plt.xticks(ticks)
    box = ax.get_position()
    ax.set_position([box.x0, box.y0 + box.height * 0.1, box.width, box.height * 0.9])
    ax.legend(loc='upper center', bbox_to_anchor=(0.5, -0.15),
          fancybox=True, shadow=True, ncol=5, fontsize=FONTSIZE * 0.7)

    plt.tight_layout()
    #plt.title(title, fontsize=FONTSIZE)
    plt.show()
    plt.clf()




#plot_thing("read", categories, result)
#plot_thing("write", categories, result)


if __name__ == "__main__":
    args = sys.argv

    if len(args) < 5:
        print("Usage: ./barplot.py {title} {x label} {y label} {folder}")
        exit(0)


    folder = args[4]

    fig, ax = plt.subplots()
    plt.xlabel(args[2], fontsize=FONTSIZE)
    plt.ylabel(args[3], fontsize=FONTSIZE)


    def k(x):
        k = lambda x: " ".join(x.split("_")[0].split(" ")[:-1])
        return k(x)


    res = os.listdir(f"{os.getcwd()}/{folder}")

    if len(args) == 6 and args[5] != '':
        res = list(filter(lambda x: args[5] == k(x), res))




    plt.rc("font", size=FONTSIZE)
    plt.rc("axes", titlesize=FONTSIZE)
    #plt.rc("xtick", labelsize=FONTSIZE)
    #plt.rc("ytick", labelsize=15)
    #plt.rc("legend", fontsize=15)



    plot_thing(res)
