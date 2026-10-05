import os
D = os.path.dirname(os.path.abspath(__file__)) + '/cells'
T = 'initial #5 $finish;'
def chain(hdr, inner, mid='localparam P = Q;'):
    return f"""module top;
  localparam Q = 1;
  if (1) begin : b
    {mid}
    for ({hdr}) begin : g
      wire [3:0] w = i;
      initial #1 $display("@%m w=%0d", w);
    end
    {inner}
  end
  {T}
endmodule
"""
I = 'genvar i = P; i < 3; i = i + 1'
cells = {
 'M12b_chain_real': chain(I, 'localparam real Q = 2.5;'),
 'M12c_chain_wide65': chain(I, "localparam [99:0] Q = 100'h1_0000_0000_0000_0002;"),
 'M12d_chain_str': chain(I, 'localparam string Q = "ab";'),
 'M12e_chain_u64max': chain(I, "localparam [63:0] Q = 64'hFFFF_FFFF_FFFF_FFFF;"),
 'M12f_chain_wide65_typedP': chain(I, "localparam [99:0] Q = 100'h1_0000_0000_0000_0002;", 'localparam [99:0] P = Q;'),
 'M12g_chain_realP': chain(I, 'localparam real Q = 2.5;', 'localparam real P = Q;'),
}
for k, v in cells.items():
    open(f'{D}/{k}.sv', 'w').write(v)
print(len(cells))
