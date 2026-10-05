module top;
  logic a = 0, b = 1, c;
  logic [1:0] y, z;
  always_comb begin
    unique if (a) y = 2'd1;
    else if (b) y = 2'd2;
  end
  always @(a or b) begin
    #0 unique if (a && b) z = 2'd3; else if (a) z = 2'd1; else if (b) z = 2'd2;
  end
  initial begin
    $dumpfile("e3.vcd"); $dumpvars(0, top);
    $monitor("t=%0t a=%b b=%b y=%0d z=%0d", $time, a, b, y, z);
    #1 a = 1; b = 0;
    #1 b = 1; a = 0;
    #1 a = 1; #0 a = 0; b = 0; b = 1;
    #1 a = 0; b = 0;
    #1 a = 1;
    #1 $finish;
  end
endmodule
