import os
D = os.path.dirname(os.path.abspath(__file__)) + '/cells'
T = 'initial #5 $finish;'
def loop(hdr, inner, outer='localparam A = 1;'):
    return f"""module top;
  {outer}
  if (1) begin : b
    for ({hdr}) begin : g
      wire [3:0] w = i;
      initial #1 $display("@%m w=%0d", w);
    end
    {inner}
  end
  {T}
endmodule
"""
I = 'genvar i = A; i < 3; i = i + 1'
cells = {
 'U8b_initA_wide65': loop(I, "localparam [99:0] A = 100'h1_0000_0000_0000_0002;"),
 'U8e_initA_u64max': loop(I, "localparam [63:0] A = 64'hFFFF_FFFF_FFFF_FFFF;"),
 'U8f_initA_x': loop(I, "localparam [3:0] A = 4'bx;"),
 'U8g_initA_div0': loop(I, "localparam A = 1/0;"),
 'U8i_condA_wide65': loop('genvar i = 0; i < A; i = i + 1', "localparam [99:0] A = 100'h1_0000_0000_0000_0002;", 'localparam A = 2;'),
 'U8j_stepA_x': loop('genvar i = 0; i < 3; i = i + A', "localparam [3:0] A = 4'bx;"),
 'U5_initA_enum': loop(I, "typedef enum {A = 2} e_t;"),
}
for k, v in cells.items():
    open(f'{D}/{k}.sv', 'w').write(v)
print(len(cells))
