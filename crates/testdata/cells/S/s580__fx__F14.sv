`timescale 1ns/1ns
class C; rand bit [3:0] x; constraint c { x inside {4'd3, 4'd5, [4'd10:4'd12]}; } endclass
class H; int a; endclass
module t;
  logic [3:0] v, e; bit [3:0] be; string s; real r; logic signed [7:0] s8;
  localparam logic [3:0] PV = 4'd12;
  localparam LQ = PV inside {4'd12, 4'd3};
  localparam string MODE = "B";
  wire w = v inside {4'd1, e, [4'd8:4'd9]};
  if (PV inside {4'd12}) begin : g initial $display("gen %b", LQ); end
  if (MODE inside {"A", "B"}) begin : gs initial $display("gen-string"); end
  function automatic logic f(input logic [3:0] a); return a inside {4'd2, 4'd4}; endfunction
  initial begin
    C o; H h, h2; int ok, okall, inset;
    o = new; h = new; h2 = h;
    v = 4'd4; e = 4'd9; be = 4'd4; s = "ab"; r = 1.5; s8 = -8'sd4;
    #1 $display("plain %b var %b cont %b fn %b str %b real %b neg %b not-range %b",
                v inside {4'd4}, v inside {e, be}, w, f(v), s inside {"ab", "cd"},
                r inside {1.5, [2.0:3.0]}, s8 inside {-8'sd4, 8'sd3}, !(v inside {[0:3]}));
    $display("fill-1-0 %b handle %b %b", v inside {'1, '0}, h inside {null, h2}, h2 inside {null});
    okall = 1; inset = 1;
    for (int i = 0; i < 20; i++) begin
      ok = o.randomize(); okall = okall & ok;
      inset = inset & (o.x == 3 || o.x == 5 || (o.x >= 10 && o.x <= 12));
    end
    $display("constraint ok %0d in-set %0d", okall, inset);
    ok = o.randomize() with { x inside {4'd11}; }; $display("with ok %0d x %0d", ok, o.x);
    case (1'b1) (v inside {4'd4, 4'd5}): $display("case item"); default: $display("case default"); endcase
    #1 $finish;
  end
endmodule
