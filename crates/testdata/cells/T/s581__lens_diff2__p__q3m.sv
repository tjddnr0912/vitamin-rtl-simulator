module top;
  logic [((4'bx100 ==? 4'b1?00) === 1'bx):0] v1;
  logic [((4'bx100 ==? 4'b1?00) !== 1'bx):0] v2;
  logic [((4'bx100 ==? 4'b1?00) && 1'b0):0] v3;
  logic [((4'bx100 ==? 4'b1?00) || 1'b1):0] v4;
  logic [(1'b1 ? 1'b1 : (4'bx100 ==? 4'b1?00)):0] v5;
  logic [(1'b0 ? (4'bx100 ==? 4'b1?00) : 1'b1):0] v6;
  logic [(&{(4'bx100 ==? 4'b1?00), 1'b0}):0] v7;
  logic [(|{(4'bx100 ==? 4'b1?00), 1'b1}):0] v8;
  logic [((4'bx100 ==? 4'b1?00) ==? 1'bx):0] v9;
  logic [$bits(4'bx100 ==? 4'b1?00):0] v10;
  logic [((4'bx100 ==? 4'b1?00) << 1):0] v11;
  logic [(4'bx100 ==? 4'bx100):0] v12;
  logic [(4'b1x00 ==? 4'b1?00):0] v13;
  logic [(4'bx100 ==? 4'bz100):0] v14;
  logic [((4'bx100 ==? 4'b1?00) ? 1'b1 : 1'b1):0] v15;
  logic [(1'b1 && (4'b1x00 ==? 4'b1?00)):0] v16;
  initial #1 $display("@ %0d %0d %0d %0d %0d %0d %0d %0d | %0d %0d %0d %0d %0d %0d %0d %0d", $bits(v1), $bits(v2), $bits(v3), $bits(v4), $bits(v5), $bits(v6), $bits(v7), $bits(v8), $bits(v9), $bits(v10), $bits(v11), $bits(v12), $bits(v13), $bits(v14), $bits(v15), $bits(v16));
endmodule
