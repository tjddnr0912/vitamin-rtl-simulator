module top;
  logic a = 0, b = 0, c = 0, e = 0;
  logic [1:0] r = 0;
  integer i;
  task automatic rec(input int d);
    if (d > 0) rec(d - 1);
    unique if (a) r = 1;
    else if (b) r = 2;
  endtask
  task automatic tj(input logic x);
    unique if (x) r = 1;
    else if (b) r = 2;
  endtask
  initial begin
    fork tj(a); join_none
    #1 for (i = 0; i < 2; i++) begin
      unique if (a) r = 1;
      else if (b) r = 2;
    end
    #1 i = 0; while (i < 2) begin unique if (a) r = 1; else if (b) r = 2; i++; end
    #1 repeat (2) unique if (a) r = 1; else if (b) r = 2;
    #1 i = 0; do begin unique if (a) r = 1; else if (b) r = 2; i++; end while (i < 2);
    #1 case (c) 1'b0: unique if (a) r = 1; else if (b) r = 2; default: r = 0; endcase
    #1 unique if (!a) begin unique if (a) r = 1; else if (b) r = 2; end else if (b) r = 3;
    #1 begin : named unique if (a) r = 1; else if (b) r = 2; end
    #1 rec(2);
    #1 i = 0; forever begin unique if (a) r = 1; else if (b) r = 2; i++; if (i == 2) break; end
    #1 e = 1;
    #1 $display("t=%0t end", $time);
    #1 $finish;
  end
  for (genvar g = 0; g < 2; g++) begin : gl
    always @(posedge e) begin
      unique if (a) r = 1;
      else if (b) r = 2;
    end
  end
  initial #100 $finish;
endmodule
