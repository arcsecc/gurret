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

        temp = list(filter(lambda x: c != x.split("_")[index], temp))
    return cat



def plot_thing(s, category, result):
    print(s)
    for category in categories:
        temp = list(filter(lambda x: category in x, res))
        #data =  list(filter(lambda x: s in x, temp))

        data =  list(filter(lambda file: file.split("_")[1] == s, temp))

        data =  list(filter(lambda file: file.split("_")[0] == category, data))
        #print(f"{data}\n\n")
        #print(category)

        key=lambda x: int(x.split("_")[2])
        data.sort(key=key)

        y = []
        yerr = []
        ticks = []

        for d in data:
            arr = [float(x[:-1]) for x in open(f"{folder}/{d}").readlines()]
            #arr.sort()
            #arr = arr[1:]

            y.append(np.mean(arr))
            yerr.append( np.std(arr) / np.sqrt(len(arr)))

        x = [i + 1 for i in range(len(y))]
        # plt.plot(x, y, label=f"{category}")
        category = category.replace("normal", "native FS")

        plt.xticks(x, [2 ** x for x in x])
        plt.errorbar(x, y, label=f"{category}", yerr=yerr, linewidth=WIDTH)


    ax.xaxis.set_major_locator(ticker.MaxNLocator(integer=True))
    ax.tick_params(labelsize=FONTSIZE)
    ax.legend(fontsize=FONTSIZE)
    plt.legend(fontsize=FONTSIZE)

    #plt.tight_layout()
    # plt.title(s, fontsize=FONTSIZE)
    plt.show()
    #plt.clf()




#plot_thing("read", categories, result)
#plot_thing("write", categories, result)


if __name__ == "__main__":
    args = sys.argv
    if len(args) != 5:
        print("Usage: ./barplot.py {title} {x label} {y label} {folder}")

    folder = args[4]

    fig, ax = plt.subplots()
    plt.xlabel("file size (MB)", fontsize=FONTSIZE)
    plt.ylabel(args[3], fontsize=FONTSIZE)
    plt.rc("font", size=FONTSIZE)
    plt.rc("axes", titlesize=FONTSIZE)

    res = os.listdir(f"{os.getcwd()}/{folder}")

    categories = get_category_by_index(res, 0)
    plt.yscale("log")
    sub = get_category_by_index(res, 1)
    sub.reverse()
    ##print(sub)
    #print(categories)
    

    for s in sub:
        plot_thing(s, categories, res)
