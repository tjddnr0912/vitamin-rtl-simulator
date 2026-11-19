module sub #(parameter [7:0] Q = 8'h0F) (output [7:0] o); assign o = Q; endmodule
module top;
  reg [7:0] r;
  wire [7:0] w2;
  sub #(8'b1010_0101) u2 (.o(w2));
  initial begin
    $display("L01 %b", 4'b1x0z);
    $display("L02 %b", 4'B1X0Z);
    $display("L03 %b", 4'b1?0?);
    $display("L04 %b", 8'hxF);
    $display("L05 %b", 8'HXf);
    $display("L06 %b", 8'hz_?);
    $display("L07 %b", 8'b1_x);
    $display("L08 %b", 4 'b x);
    $display("L09 %b", 4'b 1x);
    $display("L10 %b", 4 'b 1x);
    $display("L11 %b", 8'h	x_F);
    $display("L12 %b", 4'sb1x01);
    $display("L13 %b", 4'Sb1z);
    $display("L14 %b", 8'shxF);
    $display("L15 %b", 'sb1x);
    $display("L16 %b", 'Sh?);
    $display("L17 %b", 8'dx);
    $display("L18 %b", 8'dz);
    $display("L19 %b", 8'd?);
    $display("L20 %b", 8'dX_);
    $display("L21 %b", 8'dz_);
    $display("L22 %b", 'dx);
    $display("L23 %b", 'd?);
    $display("L24 %b", 8'sdx);
    $display("L25 %b", 6'o7x);
    $display("L26 %b", 6'O?7);
    $display("L27 %b", 'b1x);
    $display("L28 %b", 'hz);
    $display("L29 %b", 'o?);
    r = 'x; $display("L30 %b", r);
    r = 'X; $display("L31 %b", r);
    r = 'z; $display("L32 %b", r);
    r = 'Z; $display("L33 %b", r);
    r = '0; $display("L34 %b", r);
    r = '1; $display("L35 %b", r);
    $display("L36 %b %b %b %b", 4'b1010, 8'hAF, 'hFF, 8'd255);
    r = 8'b0000_0101;
    casez (r) 8'b0000_?1?1: $display("L37 z-hit"); default: $display("L37 miss"); endcase
    casex (r) 8'b0000_x1x1: $display("L38 x-hit"); default: $display("L38 miss"); endcase
    $display("L39 %b", {4'bx, 4'b1?1?});
    $display("L40 %b", (r === 8'bx));
    $display("L41 %0d", $bits(4'bx1));
    $display("L42 %b", w2);
    #1 $display("L43 %b", r ^ 8'b1x?Z_0101);
    $finish;
  end
endmodule
