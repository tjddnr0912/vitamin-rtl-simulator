module top;
  logic a = 1'bx, b = 1'bz;
  logic [3:0] v = 4'b0000, w = 4'b10x0;
  logic [1:0] r = 0;
  int k = 5;
  initial begin
    #1 unique if (a) r = 1; else if (b) r = 2;
    #1 unique if (v) r = 1; else if (w) r = 2;
    #1 unique if (k inside {[1:3]}) r = 1; else if (k inside {7, 9}) r = 2;
    #1 unique if (k inside {[1:3]}) r = 1; else if (k inside {5}) r = 2;
    #1 w = 4'bxxxx; unique if (w) r = 1; else if (v) r = 2;
    #1 unique if (w[3:2] == 2'b00) r = 1; else if (v[0] | w[0]) r = 2;
    #1 unique if (k > 6) r = 1; else if (k < 2) r = 2; else if (k == 3) r = 3;
    #1 $display("t=%0t end r=%0d", $time, r);
    #1 $finish;
  end
endmodule
