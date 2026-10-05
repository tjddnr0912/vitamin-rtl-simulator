`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v = 4'b1x00; $display("lhs-x-at-wild %b", v inside {4'b1?00});
    v = 4'b1z00; $display("lhs-z-at-wild %b", v inside {4'b1?00});
    v = 4'bx100; $display("lhs-x-compared %b", v inside {4'b1?00});
    v = 4'bz100; $display("lhs-z-compared %b", v inside {4'b1?00});
    v = 4'bx100; $display("lhs-x-under-msb-wild %b", v inside {4'b?100});
    v = 4'b1x00; $display("lhs-x-under-x-digit %b", v inside {4'b1x00});
    v = 4'bxxxx; $display("lhs-all-x-all-wild %b", v inside {4'b????});
    v = 4'bzzzz; $display("lhs-all-z-all-z %b", v inside {4'bzzzz});
    v = 4'bx110; $display("lhs-x-definite-miss %b", v inside {4'b0?00});
    v = 4'bx100; $display("lhs-x-plain-element %b", v inside {4'b0100});
    $finish;
  end
endmodule
