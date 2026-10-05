`timescale 1ns/1ns
module t;
  logic [3:0] v; function automatic logic [3:0] cf(); return 4'b1?00; endfunction
  initial begin
    v=4'b1100; $display("E50 %b", v inside {cf()});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
