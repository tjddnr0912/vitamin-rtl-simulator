`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v = 4'b1100; $display("x-digit %b", v inside {4'b1x00});
    v = 4'b1100; $display("z-digit %b", v inside {4'b1z00});
    v = 4'b0100; $display("msb-q %b", v inside {4'b?100});
    v = 4'b1100; $display("msb-x %b", v inside {4'bx100});
    v = 4'b0100; $display("msb-z %b", v inside {4'bz100});
    v = 4'b1101; $display("lsb-q %b", v inside {4'b110?});
    v = 4'b1010; $display("all-q %b", v inside {4'b????});
    v = 4'b1100; $display("hex-q %b", v inside {4'h?});
    v = 4'b1100; $display("dec-x %b", v inside {4'dx});
    v = 4'b1100; $display("oct-q %b", v inside {4'o1?});
    v = 4'b0110; $display("narrow-hit %b", v inside {3'b1?0});
    v = 4'b1100; $display("narrow-ext-compared %b", v inside {3'b1?0});
    v = 4'b1100; $display("wide-zero-hi %b", v inside {6'b001?00});
    v = 4'b1100; $display("wide-one-hi %b", v inside {6'b101?00});
    v = 4'b1100; $display("wide-wild-hi %b", v inside {6'b??1?00});
    $finish;
  end
endmodule
