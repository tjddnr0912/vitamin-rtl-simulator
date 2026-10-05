`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  string s; int m;
  initial begin
    s = "am"; case (s) inside ["aa":"az"]: m = 1; "b": m = 2; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
