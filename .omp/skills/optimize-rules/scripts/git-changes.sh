#!/bin/bash
# 识别近期的结构变更：优先找与远程默认分支的 merge-base → 显示分支全量变更；
# 找不到时（如无远程、无 origin）回退到最近 15 个提交。

echo "=== 近期提交上下文 ==="
git log --oneline -30

DEFAULT_BRANCH=$(git remote show origin 2>/dev/null | awk '/HEAD branch/ {print $NF}')
if [ -n "$DEFAULT_BRANCH" ]; then
  MERGE_BASE=$(git merge-base HEAD "origin/$DEFAULT_BRANCH" 2>/dev/null)
  if [ -n "$MERGE_BASE" ]; then
    echo "=== 自分叉点 ($MERGE_BASE) 以来的文件变更 ==="
    git diff --stat "$MERGE_BASE"..HEAD
    echo "=== 自分叉点以来的提交 ==="
    git log --oneline "$MERGE_BASE"..HEAD
  else
    echo "=== 最近 15 个提交的文件变更 ==="
    git diff --stat HEAD~15..HEAD
  fi
else
  echo "=== 最近 15 个提交的文件变更 ==="
  git diff --stat HEAD~15..HEAD
fi
