`timescale 1ns/1ns
module t;
  logic [3:0] v, e; bit [3:0] be;
  initial begin
    v=4'b1100; e=4'b1x00; $display("V01 %b", v inside {e});
    v=4'b1100; e=4'b1100; $display("V02 %b", v inside {e});
    v=4'b1100; be=4'b1100; $display("V03 %b", v inside {be});
    v=4'b1000; e=4'b1z00; $display("V04 %b", v inside {e});
    v=4'b1x00; e=4'b1100; $display("V05 %b", v inside {e});
    v=4'b1100; e=4'b1x00; $display("V06 %b", v ==? e);
    v=4'b0100; e=4'b1x00; $display("V07 %b", v inside {e});
    v=4'b1100; e=4'b1x00; $display("V08 %b", v inside {4'b0000, e});
    v=4'b1100; e=4'b0000; $display("V09 %b", v inside {e, 4'b1?00});
    v=4'b1100; e=4'bxxxx; $display("V10 %b", v inside {e});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
