`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v=4'b1x00; $display("X01 %b", v inside {4'b1?00});
    v=4'bx100; $display("X02 %b", v inside {4'b1?00});
    v=4'bz100; $display("X03 %b", v inside {4'b1?00});
    v=4'b1z00; $display("X04 %b", v inside {4'b1?00});
    v=4'bx100; $display("X05 %b", v inside {4'b0000, 4'b1?00});
    v=4'bx100; $display("X06 %b", v inside {4'b?100, 4'b1?00});
    v=4'bx100; $display("X07 %b", v inside {4'b?100});
    v=4'bxxxx; $display("X08 %b", v inside {4'b????});
    v=4'bx100; $display("X09 %b", v inside {4'b0100});
    v=4'bx100; $display("X10 %b", v inside {[4'd1:4'd7]});
    v=4'b1x00; $display("X11 %b", v inside {4'b1x00});
    v=4'bx100; $display("X12 %b", v inside {4'b0?00});
    v=4'bx110; $display("X13 %b", v inside {4'b0?00});
    v=4'bx100; $display("X14 %b", v inside {4'b1?00, 4'b0110});
    v=4'bx100; $display("X15 %b", !(v inside {4'b1?00}));
    v=4'bzzzz; $display("X16 %b", v inside {4'bzzzz});
    v=4'bzzzz; $display("X17 %b", v inside {4'b0000});
    v=4'bx100; $display("X18 %b", v inside {6'b??1?00});
    v=4'bx100; $display("X19 %b", v inside {6'b011?00});
    v=4'b1100; $display("X20 %b", v inside {4'b1?00, 4'bx});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
