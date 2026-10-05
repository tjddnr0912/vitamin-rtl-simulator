task automatic ut(input logic a, input logic b, output logic [1:0] r);
  r = 0;
  unique if (a) r = 1; else if (b) r = 2;
endtask
module sub;
  logic [1:0] y;
  task automatic t(input logic a, input logic b);
    unique if (a) y = 1; else if (b) y = 2;
  endtask
  task automatic tt(input logic a, input logic b);
    unique if (a) #2 y = 1; else if (b) #3 y = 2;
  endtask
endmodule
module top;
  sub u();
  logic [1:0] gv [2]; logic [1:0] uv;
  for (genvar g = 0; g < 2; g++) begin : gb
    task automatic gt(input logic a, input logic b);
      unique if (a) gv[g] = 1; else if (b) gv[g] = 2;
    endtask
    initial #6 gt(g == 1, 1'b0);
  end
  initial begin
    #1 u.t(0, 0); $display("t=%0t hier task", $time);
    #1 top.u.t(0, 1); $display("t=%0t hier task hit y=%0d", $time, u.y);
    #1 ut(0, 0, uv); $display("t=%0t unit task", $time);
    #1 u.tt(0, 0); $display("t=%0t timed miss", $time);
    #1 u.tt(0, 1); $display("t=%0t timed hit y=%0d", $time, u.y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
