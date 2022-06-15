#!/bin/python3

import numpy as np
import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import os

from config import *
import sys


def get_category_by_index(l: list[int], index: int) -> list[str]:
    cat = []
    temp = l.copy()
    while len(temp) != 0:
        first = temp[0]
        c = first.split("_")[index]
        cat.append(c)

        temp = list(filter(lambda x: c not in x, temp))
    return cat



FLAG=False

def plot_thing(s, category, result):
    print(s)
    for category in categories:
        temp = list(filter(lambda x: category in x, res))
        data =  list(filter(lambda file: file.split("_")[0] == category, temp))
        data =  list(filter(lambda x: s in x, temp))

        key=lambda x: int(x.split("_")[2])
        data.sort(key=key)

        y = []
        yerr = []

        for d in data:
            arr = [float(x[:-1]) for x in open(f"{folder}/{d}").readlines()]
            arr.sort()
            arr = arr[5:-5]

            y.append(np.mean(arr))
            yerr.append( np.std(arr) / np.sqrt(len(arr)))

        x = [i + 1 for i in range(len(y))]
        plt.xticks(x, [2 ** x for x in x])
        plt.errorbar(x, y, label=f"{category}", yerr=yerr, linewidth=WIDTH)



    lim =(0, 0)
    global FLAG
    if FLAG:
        lim = (0.015, 0.018)
        FLAG = not FLAG
    else:
        lim = (0.004, 0.006)
    plt.ylim((lim))
    ax.xaxis.set_major_locator(ticker.MaxNLocator(integer=True))
    ax.tick_params(labelsize=FONTSIZE)
    ax.legend(fontsize=FONTSIZE)
    plt.legend(fontsize=FONTSIZE)

    plt.show()




#plot_thing("read", categories, result)
#plot_thing("write", categories, result)


if __name__ == "__main__":
    args = sys.argv
    if len(args) != 5:
        print("Usage: ./barplot.py {title} {x label} {y label} {folder}")

    folder = args[4]

    fig, ax = plt.subplots()
    plt.xlabel("file size (Bytes)", fontsize=FONTSIZE)
    plt.ylabel(args[3], fontsize=FONTSIZE)

    res = os.listdir(f"{os.getcwd()}/{folder}")

    categories = get_category_by_index(res, 0)
    sub = get_category_by_index(res, 1)
    sub.reverse()

    for s in sub:
        plot_thing(s, categories, res)
