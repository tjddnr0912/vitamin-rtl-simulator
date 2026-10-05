localparam int K = 1;
task automatic t(input int x, output int y);
  unique if (x == K) y = 1; else if (x == 2) y = 2;
endtask
