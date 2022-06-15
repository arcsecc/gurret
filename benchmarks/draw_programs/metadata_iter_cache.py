#!/bin/python3

import numpy as np
import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import os
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



def plot_thing(data: list[str], title: str = "test2"):
    data.sort(key=lambda x: int(x.split("_")[2]))

    for d in data:

        label = d.split("_")[0]

        y = [float(x[:-1]) for x in open(f"{folder}/{d}").readlines()]
        x = [i + 1 for i in range(len(y))]
        plt.plot(x, y, label=f"{label}")


    plt.legend()

    ax.xaxis.set_major_locator(ticker.MaxNLocator(integer=True))

    plt.title(title)
    plt.show()
    plt.clf()




#plot_thing("read", categories, result)
#plot_thing("write", categories, result)


if __name__ == "__main__":
    args = sys.argv
    if len(args) != 5:
        print("Usage: ./barplot.py {title} {x label} {y label} {folder}")

    fil = args[5]
    folder = args[4]

    fig, ax = plt.subplots()
    plt.xlabel(args[2])
    plt.ylabel(args[3])

    res = os.listdir(f"{os.getcwd()}/{folder}")
    res = list(filter(lambda x: fil in x, res))

    plot_thing(res)
